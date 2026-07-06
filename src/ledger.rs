use crate::CacheDirectory;
use crate::Recipe;
use crate::State;
use anyhow::Context;
use anyhow::bail;
use fn_error_context::context;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
pub(crate) struct Ledger {
    pub files: Box<[Box<Path>]>,
}

impl Ledger {
    #[context(
        "creating a ledger of the target directory `{}`",
        recipe.directories.target(recipe, state).map_or(Path::new("<unknown>"), CacheDirectory::path).display()
    )]
    pub(crate) fn new(recipe: &Recipe, state: &State) -> anyhow::Result<Ledger> {
        let Some(target_directory) = recipe.directories.target(recipe, state)?.as_populated()
        else {
            // Perhaps a big aggressive.
            bail!("cannot create a ledger for an directory");
        };

        let mut files = Vec::new();

        for entry in WalkDir::new(target_directory) {
            let entry = entry.context("walking the directory")?;

            if entry.file_type().is_dir() {
                continue;
            }

            files.push(entry.into_path().strip_prefix(target_directory)?.into());
        }

        Ok(Ledger {
            files: files.into_boxed_slice(),
        })
    }
}
