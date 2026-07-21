mod git;
mod index;
mod tar;

use crate::DownloadLock;
use crate::HostPath;
use crate::Recipe;
use crate::State;
use fn_error_context::context;
use git::download_git;
use tar::download_tarball;
use tracing::info;

pub(crate) use git::resolve_commit;
pub(crate) use index::IndexedFile;
pub(crate) use index::find_in_index;
pub(crate) use tar::detect_tarball_compression;
pub(crate) use tar::split_tarball_file_name;

pub(crate) fn ensure_downloaded(recipe: &Recipe, state: &State) -> anyhow::Result<()> {
    let name = &recipe.name;

    recipe
        .directories
        .source(recipe.download_lock(state)?, state)?
        .as_populated_then_run_or_populate_with(
            |_| info!("using the cached source directory for the `{name}` recipe"),
            |into| {
                info!("downloading the source code for the `{name}` recipe");
                download(recipe, into, state)?;
                info!("downloaded the source code for the `{name}` recipe");
                anyhow::Ok(())
            },
        )
}

#[context("downloading the source code for the `{}` recipe", recipe.name)]
fn download(recipe: &Recipe, source_directory: &HostPath, state: &State) -> anyhow::Result<()> {
    match recipe.download_lock(state)? {
        DownloadLock::None => (),
        DownloadLock::Git { url, commit } => {
            download_git(url, *commit, source_directory, &recipe.directories, state)?;
        }
        DownloadLock::Tarball {
            virtual_url: _,
            compression,
            real_url,
        } => {
            download_tarball(real_url, *compression, source_directory)?;
        }
    }

    Ok(())
}
