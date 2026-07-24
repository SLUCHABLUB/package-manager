use crate::CacheDirectory;
use crate::HostPath;
use crate::Recipe;
use crate::State;
use crate::TargetPath;
use anyhow::Context;
use fn_error_context::context;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use tracing::warn;
use walkdir::WalkDir;

// TODO: Make this opaque.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct SystemLedger {
    #[serde(flatten)]
    pub recipes: HashMap<Box<str>, PackageLedger>,
}

impl SystemLedger {
    pub(crate) fn new() -> SystemLedger {
        SystemLedger::default()
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = (&str, &TargetPath)> {
        self.recipes
            .iter()
            .flat_map(|(recipe, ledger)| ledger.files().map(|file| (&**recipe, file)))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct PackageLedger(Box<[Box<TargetPath>]>);

impl PackageLedger {
    #[context(
        // TODO: Figure out how to do this cleanly.
        "creating a ledger of the target directory `{:?}`",
        recipe.directories.target(recipe, state).map(CacheDirectory::path)
    )]
    pub(crate) fn new(recipe: &Recipe, state: &State) -> anyhow::Result<PackageLedger> {
        let target_directory = recipe.directories.target(recipe, state)?;
        let Some(target_directory) = target_directory.as_populated() else {
            warn!(
                "creating a ledger for the empty directory `{}`",
                target_directory.path()
            );
            return Ok(PackageLedger::default());
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

        Ok(PackageLedger(files.into_boxed_slice()))
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = &TargetPath> {
        let PackageLedger(files) = self;
        files.iter().map(|file| &**file)
    }
}
