use crate::recipe::DownloadSource;
use crate::recipe::Recipe;
use anyhow::Context;
use bstr::BStr;
use bstr::ByteSlice as _;
use fs_err::create_dir_all;
use fs_err::remove_dir_all;
use gix::ObjectId;
use gix::progress::Discard;
use gix::protocol::handshake::Ref;
use gix::remote::Direction;
use gix::remote::ref_map::Options;
use semver::Version;
use semver::VersionReq;
use std::io;
use std::path::Path;
use tracing::info;
use tracing::warn;

pub fn download(recipe: &Recipe, destination: &Path) -> anyhow::Result<()> {
    match &recipe.download.source {
        DownloadSource::Github { repository } => {
            download_github(repository, &recipe.download.version, destination)
                .with_context(|| format!("downloading github repository {repository}"))
        }
    }
}

fn make_empty_directory(directory: &Path) -> anyhow::Result<()> {
    match remove_dir_all(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }?;
    create_dir_all(directory)?;

    Ok(())
}

fn download_github(
    repository_path: &str,
    target_version: &VersionReq,
    destination: &Path,
) -> anyhow::Result<()> {
    make_empty_directory(destination).context("preparing destination directory")?;

    let progress = Discard;

    let repository = gix::init(destination).context("initialising the repository")?;

    let remote = repository
        .remote_at(format!("https://github.com/{repository_path}.git"))
        .context("adding the remote")?;

    let connection = remote
        .connect(Direction::Fetch)
        .context("connecting to the repository")?;

    let (references, _handshake) = connection
        .ref_map(progress, Options::default())
        .context("fetching references")?;

    let mut best_commit = None;
    let mut best_version = None;

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

        if target_version.matches(&version)
            && best_version.as_ref().is_none_or(|best| version > *best)
        {
            if !version.build.is_empty() {
                info!("skipping version `{version}` due to build metadata");
                continue;
            }

            best_commit = Some(commit);
            best_version = Some(version);
        }
    }

    let commit = best_commit
        .with_context(|| format!("could not find a tag matching the version {target_version}"))?;

    info!("found commit: {commit:?}");

    Ok(())
}

fn to_tag(reference: &Ref) -> Option<(&BStr, ObjectId)> {
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
        Ref::Symbolic { .. } | Ref::Unborn { .. } => return None,
    };

    let name = name.strip_prefix(b"refs/tags/")?;

    Some((name.as_bstr(), *commit))
}

fn parse_version(tag_name: &BStr) -> anyhow::Result<Version> {
    let tag_name = str::from_utf8(tag_name).context("parsing the tag name as utf-8")?;

    let version = tag_name
        .strip_prefix("v")
        .context("parsing tag as `v`-prefixed version")?;

    Version::parse(version).with_context(|| format!("parsing `{version}` as a semantic version"))
}
