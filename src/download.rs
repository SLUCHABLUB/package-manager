use crate::Version;
use crate::directories::RecipeDirectories;
use crate::fs;
use crate::recipe::Download;
use crate::recipe::Recipe;
use anyhow::Context;
use bstr::BStr;
use bstr::ByteSlice as _;
use gix::ObjectId;
use gix::progress::Discard;
use gix::protocol::handshake::Ref;
use gix::remote::Direction;
use gix::remote::fetch::Shallow;
use gix::remote::ref_map;
use gix::worktree::state::checkout;
use non_zero::non_zero;
use std::sync::atomic::AtomicBool;
use tracing::info;
use tracing::warn;

pub fn download(recipe: &Recipe, directories: &RecipeDirectories) -> anyhow::Result<()> {
    match &recipe.download {
        Download::Github { repository } => {
            download_github(repository, &recipe.version, directories)
                .with_context(|| format!("downloading github repository {repository}"))
        }
    }
}

struct VersionTag<'name> {
    name: &'name BStr,
    commit: ObjectId,
    version: Version,
}

fn download_github(
    repository_path: &str,
    target_version: &Version,
    directories: &RecipeDirectories,
) -> anyhow::Result<()> {
    fs::make_empty_directory(&directories.repository)
        .context("preparing the repository location")?;
    fs::make_empty_directory(&directories.source).context("preparing the destination directory")?;

    let url = format!("https://github.com/{repository_path}.git");

    let mut progress = Discard;
    let interrupt = AtomicBool::new(false);

    let repository = gix::init_bare(&directories.repository)
        .context("initialising the destination repository")?;

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

    let tag = best_tag
        .with_context(|| format!("could not find a tag matching the version {target_version}"))?;

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
        &directories.source,
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

fn parse_version(tag_name: &BStr) -> anyhow::Result<Version> {
    let tag_name = str::from_utf8(tag_name).context("parsing the tag name as utf-8")?;

    let version = tag_name
        .strip_prefix("v")
        .context("parsing tag as `v`-prefixed version")?;

    Ok(Version::from(version))
}
