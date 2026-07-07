mod git;
mod index;
mod tar;

use crate::DownloadLock;
use crate::Recipe;
use crate::State;
use fn_error_context::context;
use git::download_git;
use std::path::Path;
use tar::download_tarball;

pub(crate) use git::resolve_commit;
pub(crate) use index::find_in_index;
pub(crate) use tar::detect_tarball_compression;
pub(crate) use tar::split_tarball_file_name;

#[context("downloading the source code for the `{}` recipe", recipe.name)]
pub(crate) fn download(
    recipe: &Recipe,
    source_directory: &Path,
    state: &State,
) -> anyhow::Result<()> {
    match recipe.download_lock(state)? {
        DownloadLock::None => (),
        DownloadLock::Git { url, commit } => {
            download_git(url, *commit, source_directory, &recipe.directories, state)?;
        }
        DownloadLock::Tarball { url, compression } => {
            download_tarball(url, *compression, source_directory)?;
        }
    }

    Ok(())
}
