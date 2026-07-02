use crate::RecipeDirectories;
use crate::directories::CacheDirectory;
use anyhow::Context;
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
        directories.target().map_or(Path::new("<unknown>"), CacheDirectory::path).display()
    )]
    pub(crate) fn new(directories: &RecipeDirectories) -> anyhow::Result<Ledger> {
        let target_directory = directories.target()?.path();

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
