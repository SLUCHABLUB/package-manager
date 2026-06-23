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
    pub fn from_target_directory(directory: &Path) -> anyhow::Result<Ledger> {
        let mut files = Vec::new();

        for entry in WalkDir::new(directory) {
            let entry = entry.context("walking the directory")?;

            if entry.file_type().is_dir() {
                continue;
            }

            files.push(entry.into_path().into_boxed_path());
        }

        Ok(Ledger {
            files: files.into_boxed_slice(),
        })
    }
}
