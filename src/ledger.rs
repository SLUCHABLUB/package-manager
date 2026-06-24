use crate::RecipeDirectories;
use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
pub struct Ledger {
    files: Box<[Box<Path>]>,
}

impl Ledger {
    pub fn new(directories: &RecipeDirectories) -> anyhow::Result<Ledger> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&directories.target) {
            let entry = entry.context("walking the directory")?;

            if entry.file_type().is_dir() {
                continue;
            }

            files.push(entry.into_path().strip_prefix(&directories.target)?.into());
        }

        Ok(Ledger {
            files: files.into_boxed_slice(),
        })
    }
}
