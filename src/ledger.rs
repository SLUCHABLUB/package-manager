use crate::CacheDirectory;
use crate::HostPath;
use crate::Recipe;
use crate::State;
use crate::TargetPath;
use anyhow::Context;
use fn_error_context::context;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;
use walkdir::WalkDir;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Ledger {
    pub files: Box<[Box<TargetPath>]>,
}

impl Ledger {
    #[context(
        // TODO: Figure out how to do this cleanly.
        "creating a ledger of the target directory `{:?}`",
        recipe.directories.target(recipe, state).map(CacheDirectory::path)
    )]
    pub(crate) fn new(recipe: &Recipe, state: &State) -> anyhow::Result<Ledger> {
        let target_directory = recipe.directories.target(recipe, state)?;
        let Some(target_directory) = target_directory.as_populated() else {
            warn!(
                "creating a ledger for the empty directory `{}`",
                target_directory.path()
            );
            return Ok(Ledger::default());
        };

        let mut files = Vec::new();

        for entry in WalkDir::new(target_directory) {
            let entry = entry.context("walking the directory")?;

            let path =
                HostPath::new(entry.path()).expect("the items of WalkDir should be absolute");

            if entry.file_type().is_dir() {
                continue;
            }

            files.push(TargetPath::from_path_and_root(path, target_directory));
        }

        Ok(Ledger {
            files: files.into_boxed_slice(),
        })
    }
}
