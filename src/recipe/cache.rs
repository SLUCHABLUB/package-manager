use crate::DownloadLock;
use crate::HostDirectories;
use crate::HostPath;
use crate::Recipe;
use crate::hash;
use anyhow::Context as _;
use anyhow::bail;
use fs_err::create_dir_all;
use fs_err::read_dir;
use fs_err::remove_dir_all;
use gix::Repository;
use rapidhash::v3::rapidhash_v3;
use std::io;
use std::path::Path;
use tracing::info;
use tracing::warn;
use url::Url;

pub(crate) fn find_cached_download_lock_or_create(
    recipe: &Recipe,
    host: &HostDirectories,
) -> anyhow::Result<DownloadLock> {
    let path = host.download_locks.with_suffix(format!(
        "{}-{}.toml",
        hash((recipe.version(), recipe.download_data())),
        recipe.name()
    ));

    Ok(if path.exists() {
        info!("using the cached lock");
        DownloadLock::read_from(&path, host)?
    } else {
        let lock = recipe.lock(host)?;
        lock.write_to(&path)?;
        lock
    })
}

pub(super) fn find_cached_repository_or_initialise(
    url: &Url,
    host: &HostDirectories,
) -> anyhow::Result<Repository> {
    let path = host.repositories.with_suffix(encode_url(url));

    if is_directory_populated(&path)? {
        info!("using the cached repository");
        gix::open(&*path).context("opening the cached repository")
    } else {
        create_dir_all(&path)?;
        gix::init_bare(&path).context("initialising the git repository")
    }
}

/// A downloaded (and extracted) source tree.
#[derive(Debug)]
pub(crate) struct Source(pub Box<HostPath>);

impl Source {
    pub(crate) fn find_cached(
        download_lock: &DownloadLock,
        host: &HostDirectories,
    ) -> anyhow::Result<Result<Source, Box<HostPath>>> {
        let path = match download_lock {
            DownloadLock::None => host.sources.with_suffix("none"),
            DownloadLock::Git {
                url,
                commit,
                repository: _,
            } => {
                let mut suffix = encode_url(url);
                suffix += &commit.to_string();
                host.sources.with_suffix(suffix)
            }
            DownloadLock::Tarball {
                virtual_url,
                compression: _,
                real_url,
            } => {
                let url = virtual_url.as_ref().unwrap_or(real_url);
                host.sources.with_suffix(encode_url(url))
            }
        };

        Ok(if is_directory_populated(&path)? {
            info!("using the cached source tree");
            Ok(Source(path))
        } else {
            Err(path)
        })
    }
}

#[derive(Debug)]
pub(crate) struct BuildRoot(pub Box<HostPath>);

impl BuildRoot {
    pub(crate) fn new(source: Source, recipe: &Recipe) -> anyhow::Result<BuildRoot> {
        let Source(source) = source;

        Ok(match &recipe.build_data().directory {
            Some(directory) => {
                let path = source.with_suffix(directory);
                if !path.is_dir() {
                    bail!(
                        "the directory `{}` does not exist in the source for {recipe}",
                        directory.display()
                    );
                }
                BuildRoot(path)
            }
            None => BuildRoot(source),
        })
    }
}

#[derive(Debug)]
pub(crate) struct BuildWorkingDirectory(pub Box<HostPath>);

impl BuildWorkingDirectory {
    pub(crate) fn new(
        recipe: &Recipe,
        host: &HostDirectories,
    ) -> anyhow::Result<BuildWorkingDirectory> {
        // TODO: Base this on the recipe hash.
        let path = host.working.with_suffix(recipe.name());

        make_empty_directory(&path)?;

        Ok(BuildWorkingDirectory(path))
    }
}

/// A built image.
#[derive(Debug)]
pub(crate) struct Image(pub Box<HostPath>);

impl Image {
    pub(crate) fn find_cached(
        recipe: &Recipe,
        host: &HostDirectories,
    ) -> anyhow::Result<Result<Image, Box<HostPath>>> {
        // TODO: Base this on the recipe hash.
        let path = host.images.with_suffix(recipe.name());

        Ok(if is_directory_populated(&path)? {
            info!("using the cached package image");
            Ok(Image(path))
        } else {
            Err(path)
        })
    }
}

fn encode_url(url: &Url) -> String {
    let hash = rapidhash_v3(url.as_str().as_bytes());

    if let Some(human_readable_part) = url
        .path_segments()
        .and_then(Iterator::last)
        .or_else(|| url.domain())
    {
        format!("{hash:x}-{human_readable_part}")
    } else {
        warn!("could not determine a human readable component of `{url}`");
        format!("{hash:x}")
    }
}

fn make_empty_directory(directory: impl AsRef<Path>) -> anyhow::Result<()> {
    let directory = directory.as_ref();

    match remove_dir_all(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }?;
    create_dir_all(directory)?;

    Ok(())
}

fn is_directory_populated(directory: impl AsRef<Path>) -> anyhow::Result<bool> {
    let directory = directory.as_ref();

    if !directory.exists() {
        return Ok(false);
    }

    if !directory.is_dir() {
        return Ok(false);
    }

    // TODO: Should we consider it populated if we get an error from the iterator?
    Ok(read_dir(directory)?.find(Result::is_ok).is_some())
}
