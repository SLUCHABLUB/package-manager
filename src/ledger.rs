use fs_err::read_dir;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Ledger {
    files: Box<[Box<Path>]>,
}

impl Ledger {
    pub fn from_target_directory(directory: &Path) -> anyhow::Result<Ledger> {
        let mut files = Vec::new();

        for entry in read_dir(directory)? {
            // TODO: recurse
            files.push(entry?.path().into_boxed_path());
        }

        Ok(Ledger {
            files: files.into_boxed_slice(),
        })
    }
}
