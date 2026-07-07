use crate::RecipeDirectories;
use crate::Resolver;
use crate::State;
use crate::Version;
use crate::VersionRequirement;
use anyhow::Context as _;
use anyhow::bail;
use bstr::BStr;
use bstr::ByteSlice as _;
use fn_error_context::context;
use gix::ObjectId;
use gix::Repository;
use gix::progress::Discard;
use gix::protocol::handshake::Ref;
use gix::remote::Direction;
use gix::remote::fetch::Shallow;
use gix::remote::ref_map;
use gix::worktree::state::checkout;
use non_zero::non_zero;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tracing::info;
use tracing::warn;
use url::Url;

#[context("downloading the git repository at `{url}`")]
pub(in crate::download) fn download_git(
    url: &Url,
    commit: ObjectId,
    source_directory: &Path,
    directories: &RecipeDirectories,
    state: &State,
) -> anyhow::Result<()> {
    let interrupt = AtomicBool::new(false);
    let mut progress = Discard;

    let repository = repository(directories, url, state)?;

    let remote = repository
        .remote_at(url.as_str())
        .context("adding the remote")?;

    // A ref-spec of only `<commit>` will fetch only said tag into `FETCH_HEAD`.
    let remote = remote
        .with_refspecs([commit.to_string().as_bytes()], Direction::Fetch)
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
        .find_commit(commit)
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

#[context("resolving the version {version} to a commit in the repository at `{repository_url}`")]
pub(crate) fn resolve_commit(
    repository_url: &Url,
    version: &VersionRequirement,
    directories: &RecipeDirectories,
    state: &State,
) -> anyhow::Result<ObjectId> {
    let mut progress = Discard;

    let repository = repository(directories, repository_url, state)?;

    let remote = repository
        .remote_at(repository_url.as_str())
        .context("adding the remote")?;

    let connection = remote
        .connect(Direction::Fetch)
        .context("connecting to the repository")?;

    let (references, _handshake) = connection
        .ref_map(&mut progress, ref_map::Options::default())
        .context("fetching references")?;

    let mut resolver = Resolver::from_requirement(version);

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

        resolver.add_option(commit, version);
    }

    let Some(commit) = resolver.best() else {
        bail!("could not find a tag matching the version {version}");
    };

    Ok(commit)
}

fn repository(
    directories: &RecipeDirectories,
    url: &Url,
    state: &State,
) -> anyhow::Result<Repository> {
    directories
        .repository_from_git_url(url, state)?
        .as_populated_then_try_or_populate_with(
            |path| {
                info!("using the cached repository");
                gix::open(path).context("opening the cached repository")
            },
            |path| gix::init_bare(path).context("initialising the git repository"),
        )
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
