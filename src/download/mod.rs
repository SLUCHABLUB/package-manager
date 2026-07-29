mod git;
mod index;
mod tar;

use crate::DownloadLock;
use crate::Recipe;
use fn_error_context::context;
use git::download_git;
use tar::download_tarball;

use crate::HostDirectories;
use crate::Source;
use fs_err::create_dir_all;
pub(crate) use git::resolve_commit;
pub(crate) use index::IndexedFile;
pub(crate) use index::find_in_index;
pub(crate) use tar::detect_tarball_compression;
pub(crate) use tar::split_tarball_file_name;
use tracing::info;

#[context("downloading the source code for {recipe}")]
pub(crate) fn download(
    recipe: &Recipe,
    download_lock: &DownloadLock,
    host: &HostDirectories,
) -> anyhow::Result<Source> {
    let source_directory = match Source::find_cached(download_lock, host)? {
        Ok(cached) => return Ok(cached),
        Err(uninitialised) => uninitialised,
    };

    info!("downloading {}", recipe.name());

    if !matches!(download_lock, DownloadLock::None) {
        create_dir_all(&source_directory)?;
    }

    let source = match download_lock {
        DownloadLock::None => Source(source_directory),
        DownloadLock::Git {
            repository,
            url,
            commit,
        } => download_git(repository, url, *commit, source_directory)?,
        DownloadLock::Tarball {
            virtual_url: _,
            compression,
            real_url,
        } => download_tarball(real_url, *compression, source_directory)?,
    };

    info!("downloaded {}", recipe.name());

    Ok(source)
}
