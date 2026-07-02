use crate::Version;
use crate::VersionRequirement;
use crate::directories::RecipeDirectories;
use crate::recipe::Compression;
use crate::recipe::Download;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use bstr::BStr;
use bstr::ByteSlice as _;
use fn_error_context::context;
use gix::ObjectId;
use gix::progress::Discard;
use gix::protocol::handshake::Ref;
use gix::remote::Direction;
use gix::remote::fetch::Shallow;
use gix::remote::ref_map;
use gix::worktree::state::checkout;
use lzma_rs::xz_decompress;
use non_zero::non_zero;
use std::io::Cursor;
use std::str::from_utf8;
use std::sync::atomic::AtomicBool;
use tar::Archive;
use tl::Node;
use tl::ParserOptions;
use tracing::info;
use tracing::warn;
use url::Url;

#[context("downloading the source code for the `{}` recipe", recipe.name)]
pub(crate) fn download(recipe: &Recipe, directories: &RecipeDirectories) -> anyhow::Result<()> {
    match &recipe.download {
        Download::Github {
            version,
            repository,
        } => download_github(repository, version, directories)?,
        Download::Tarball { url, compression } => {
            let Some(compression) = compression.or_else(|| detect_compression(url.as_str())) else {
                bail!("could not detect compression of tarball at `{url}`");
            };

            download_tarball(url, compression, directories)?
        }
        Download::TarballIndex {
            url,
            version,
            file_name_prefix,
        } => {
            let (tarball_url, compression) = find_in_index(url, version, file_name_prefix)?;

            download_tarball(&tarball_url, compression, directories)?
        }
    }

    Ok(())
}

struct Resolver<'requirement, T> {
    requirement: &'requirement VersionRequirement,
    best: Option<(T, Version)>,
}

impl<T> Resolver<'_, T> {
    fn from_requirement(requirement: &VersionRequirement) -> Resolver<'_, T> {
        Resolver {
            requirement,
            best: None,
        }
    }

    fn add_option(&mut self, value: T, version: Version) {
        if version.satisfies(self.requirement)
            && self
                .best
                .as_ref()
                .is_none_or(|(_value, best_version)| version > *best_version)
        {
            self.best = Some((value, version))
        }
    }

    fn best(self) -> Option<T> {
        self.best.map(|(value, _version)| value)
    }
}

struct VersionTag<'name> {
    name: &'name BStr,
    commit: ObjectId,
    version: Version,
}

#[context("downloading the github repository {repository_path}")]
fn download_github(
    repository_path: &str,
    target_version: &VersionRequirement,
    directories: &RecipeDirectories,
) -> anyhow::Result<()> {
    let Some(source_directory) = directories.source()?.as_unpopulated() else {
        info!("using the cached source");
        return Ok(());
    };

    let url = format!("https://github.com/{repository_path}.git");

    let mut progress = Discard;
    let interrupt = AtomicBool::new(false);

    let repository_directory = directories.repository()?;

    let repository = if repository_directory.is_populated() {
        info!("using the cached repository");
        gix::open(repository_directory.path()).context("opening the cached repository")?
    } else {
        gix::init_bare(repository_directory.path())
            .context("initialising the destination repository")?
    };

    let remote = repository.remote_at(url).context("adding the remote")?;

    let connection = remote
        .connect(Direction::Fetch)
        .context("connecting to the repository")?;

    let (references, _handshake) = connection
        .ref_map(&mut progress, ref_map::Options::default())
        .context("fetching references")?;

    let mut best_tag: Option<VersionTag<'_>> = None;

    for reference in &references.remote_refs {
        let Some((tag_name, commit)) = to_tag(reference) else {
            continue;
        };

        let version = match parse_version(tag_name) {
            Ok(version) => version,
            Err(error) => {
                warn!("skipping the tag `{tag_name}` due to an error: {:#}", error);
                continue;
            }
        };

        let tag = VersionTag {
            name: tag_name,
            commit,
            version,
        };

        if tag.version.satisfies(target_version)
            && best_tag
                .as_ref()
                .is_none_or(|best| tag.version > best.version)
        {
            best_tag = Some(tag);
        }
    }

    let Some(tag) = best_tag else {
        bail!("could not find a tag matching the version {target_version}");
    };

    info!(
        "using version {} which corresponds to commit {}",
        tag.version, tag.commit
    );

    // A ref-spec of only `<tag>` will fetch only said tag into `FETCH_HEAD`.
    let remote = remote
        .with_refspecs([tag.name], Direction::Fetch)
        .context("setting ref-specs")?;

    let connection = remote
        .connect(Direction::Fetch)
        .context("connecting to the repository")?;

    let _outcome = connection
        .prepare_fetch(&mut progress, ref_map::Options::default())
        .context("preparing object fetch")?
        .with_shallow(Shallow::DepthAtRemote(non_zero!(1)))
        .receive(&mut progress, &interrupt)
        .context("fetching the repository")?;

    let commit = repository
        .find_commit(tag.commit)
        .context("finding the commit")?;

    let tree = commit.tree().context("finding the tree")?;

    let mut index = repository
        .index_from_tree(&tree.id)
        .context("finding the index")?;

    let file_counter = Discard;
    let byte_counter = Discard;

    let _outcome = checkout(
        &mut index,
        source_directory,
        repository.objects.clone(),
        &file_counter,
        &byte_counter,
        &interrupt,
        checkout::Options::default(),
    )
    .context("checking out the tree")?;

    Ok(())
}

fn to_tag(reference: &Ref) -> Option<(&BStr, ObjectId)> {
    const TAG_PREFIX: &[u8] = b"refs/tags/";

    let (name, commit) = match reference {
        Ref::Direct {
            full_ref_name,
            object,
        }
        | Ref::Peeled {
            full_ref_name,
            tag: _,
            object,
        } => (full_ref_name, object),
        // These shouldn't be able to be tags.
        Ref::Symbolic { full_ref_name, .. } | Ref::Unborn { full_ref_name, .. } => {
            debug_assert!(!full_ref_name.starts_with(TAG_PREFIX));
            return None;
        }
    };

    let name = name.strip_prefix(TAG_PREFIX)?.as_bstr();

    Some((name, *commit))
}

#[context("parsing a version from the `{tag_name}` tag")]
fn parse_version(tag_name: &BStr) -> anyhow::Result<Version> {
    let tag_name = str::from_utf8(tag_name).context("parsing the tag name as utf-8")?;

    let version = tag_name
        .strip_prefix("v")
        .context("parsing tag as `v`-prefixed version")?;

    Ok(Version::from(version))
}

#[context("downloading the tarball from `{url}`")]
fn download_tarball(
    url: &Url,
    compression: Compression,
    directories: &RecipeDirectories,
) -> anyhow::Result<()> {
    let Some(source_directory) = directories.source()?.as_unpopulated() else {
        info!("using the cached source");
        return Ok(());
    };

    let response = reqwest::blocking::get(url.clone())?;

    let response = response.error_for_status()?;

    let compressed_bytes = response.bytes()?;

    let mut decompressed_bytes = Vec::new();

    match compression {
        Compression::None => decompressed_bytes = compressed_bytes.to_vec(),
        Compression::Xz => {
            xz_decompress(&mut &*compressed_bytes, &mut decompressed_bytes)
                .context("decompressing the tarball")?;
        }
    }

    let mut archive = Archive::new(Cursor::new(decompressed_bytes));

    archive.unpack(source_directory)?;

    Ok(())
}

fn basename_and_compression(url_or_path: &str) -> Option<(&str, Compression)> {
    let (basename, extension) = url_or_path.rsplit_once(".tar")?;

    if extension.is_empty() {
        return Some((basename, Compression::None));
    }

    let extension = extension.strip_prefix(".")?;

    if extension.contains('/') {
        return None;
    }

    let compression = match extension {
        "xz" => Compression::Xz,
        _ => {
            warn!("unknown compression extension: `.{extension}`");
            return None;
        }
    };

    Some((basename, compression))
}

fn detect_compression(url_or_path: &str) -> Option<Compression> {
    let (_basename, compression) = basename_and_compression(url_or_path)?;
    Some(compression)
}

#[context("finding a file matching version {version} in the index at `{index}`")]
fn find_in_index(
    index: &Url,
    version: &VersionRequirement,
    file_name_prefix: &str,
) -> anyhow::Result<(Url, Compression)> {
    let response = reqwest::blocking::get(index.clone())?;

    let response = response.error_for_status()?;

    // We may get redirected.
    let resolved_index = response.url().clone();

    let bytes = response.bytes()?;
    let string = from_utf8(&bytes).context("parsing the HTML as UTF-8")?;

    let dom = tl::parse(string, ParserOptions::new()).context("parsing the HTML")?;

    // TODO: Set favouring of compression types.
    let mut resolver = Resolver::from_requirement(version);

    for node in dom.nodes() {
        let Node::Tag(tag) = node else {
            continue;
        };

        if tag.name() != "a" {
            continue;
        }

        let Some(Some(file_name)) = tag.attributes().get("href") else {
            continue;
        };

        let Ok(file_name) = from_utf8(file_name.as_bytes()) else {
            continue;
        };

        let Some((basename, compression)) = basename_and_compression(file_name) else {
            continue;
        };

        let Some(version) = basename.strip_prefix(file_name_prefix) else {
            continue;
        };

        let version = Version::from(version);

        resolver.add_option((file_name, compression), version);
    }

    let Some((file_name, compression)) = resolver.best() else {
        bail!("found file matching version {version}");
    };

    let url = resolved_index
        .join(file_name)
        .context("joining the file name to the index url")?;

    info!("resolved index `{index}` with version {version} to `{url}`");

    Ok((url, compression))
}
