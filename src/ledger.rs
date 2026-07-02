use crate::RecipeDirectories;
use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
pub struct Ledger {
    pub files: Box<[Box<Path>]>,
}

impl Ledger {
    pub fn new(directories: &RecipeDirectories) -> anyhow::Result<Ledger> {
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
