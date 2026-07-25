use crate::CacheDirectory;
use crate::HostPath;
use crate::PACKAGE_NAME;
use crate::Recipe;
use crate::State;
use crate::TargetDirectories;
use crate::TargetPath;
use anyhow::Context;
use const_str::join;
use fn_error_context::context;
use fs_err as fs;
use fs_err::File;
use fs_err::create_dir_all;
use rapidhash::v3::rapidhash_v3_file;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path;
use tracing::warn;
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) struct SystemLedger {
    path: Box<TargetPath>,
    data: SystemLedgerData,
}

impl SystemLedger {
    pub(crate) fn new(target: &TargetDirectories) -> SystemLedger {
        SystemLedger {
            path: target.data().with_suffix(join!(
                &[PACKAGE_NAME, "ledger.toml"],
                path::MAIN_SEPARATOR_STR
            )),
            data: SystemLedgerData::default(),
        }
    }

    pub(crate) fn path(&self) -> &TargetPath {
        &self.path
    }

    pub(crate) fn add_recipe(&mut self, recipe: RecipeLedger) {
        self.data.recipes.push(recipe);
    }

    pub(crate) fn contains(&self, file: &TargetPath) -> bool {
        self.files().any(|(_recipe, path, _hash)| path == file)
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = (&str, &TargetPath, u64)> {
        self.data.recipes.iter().flat_map(|ledger| {
            ledger
                .hashes
                .iter()
                .map(|(file, hash)| (&*ledger.name, &**file, *hash))
        })
    }

    pub(crate) fn write_to_root(&self, root: &HostPath) -> anyhow::Result<()> {
        let host_path = self.path.with_root(root);

        if let Some(parent) = host_path.parent() {
            create_dir_all(parent)?;
        }

        let serialised = toml::to_string(&self.data).context("serialising the ledger")?;
        fs::write(host_path, serialised)?;

        Ok(())
    }

    pub(crate) fn read_from_host(target: &TargetDirectories) -> anyhow::Result<SystemLedger> {
        let mut ledger = SystemLedger::new(target);

        let serialised = match fs::read_to_string(ledger.path.to_host_path()) {
            // We return an empty ledger if the file is not found.
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ledger),
            result => result?,
        };
        ledger.data = toml::from_str(&serialised).context("deserialising the ledger")?;

        Ok(ledger)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SystemLedgerData {
    recipes: Vec<RecipeLedger>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecipeLedger {
    name: Box<str>,
    // TODO: Make this a sequence on structs when serialising.
    hashes: HashMap<Box<TargetPath>, u64>,
}

impl RecipeLedger {
    #[context(
        // TODO: Figure out how to do this cleanly.
        "creating a ledger of the target directory `{:?}`",
        recipe.directories.target(recipe, state).map(CacheDirectory::path)
    )]
    pub(crate) fn new(recipe: &Recipe, state: &State) -> anyhow::Result<RecipeLedger> {
        let target_directory = recipe.directories.target(recipe, state)?;
        let Some(target_directory) = target_directory.as_populated() else {
            warn!(
                "creating a ledger for the empty directory `{}`",
                target_directory.path()
            );
            return Ok(RecipeLedger::empty(recipe.name.clone()));
        };

        let mut hashes = HashMap::new();

        for entry in WalkDir::new(target_directory) {
            let entry = entry.context("walking the directory")?;

            let path =
                HostPath::new(entry.path()).expect("the items of WalkDir should be absolute");

            if entry.file_type().is_dir() {
                continue;
            }

            let file = File::open(path)?;
            let hash = rapidhash_v3_file(file)?;

            hashes.insert(TargetPath::from_path_and_root(path, target_directory), hash);
        }

        Ok(RecipeLedger {
            name: recipe.name.clone(),
            hashes,
        })
    }

    fn empty(recipe: Box<str>) -> RecipeLedger {
        RecipeLedger {
            name: recipe,
            hashes: HashMap::new(),
        }
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = (&TargetPath, u64)> {
        self.hashes.iter().map(|(file, hash)| (&**file, *hash))
    }
}
