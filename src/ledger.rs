use crate::HostPath;
use crate::Image;
use crate::PACKAGE_NAME;
use crate::Recipe;
use crate::TargetDirectories;
use crate::TargetPath;
use crate::Version;
use crate::VersionRequirement;
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

    pub(crate) fn contains(
        &self,
        package_name: &str,
        version_requirement: &VersionRequirement,
    ) -> bool {
        self.data.packages.iter().any(|package| {
            &*package.name == package_name && package.version.satisfies(version_requirement)
        })
    }

    pub(crate) fn add_image(&mut self, recipe: ImageLedger) {
        self.data.packages.push(recipe);
    }

    pub(crate) fn hash(&self, file: &TargetPath) -> Option<u64> {
        self.files()
            .find_map(|(_recipe, path, hash)| (path == file).then_some(hash))
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = (&str, &TargetPath, u64)> {
        self.data.packages.iter().flat_map(|ledger| {
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
    packages: Vec<ImageLedger>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ImageLedger {
    /// The name of the corresponding package.
    name: Box<str>,
    /// The version of the corresponding package.
    version: Version,
    // TODO: Make this a sequence on structs when serialising.
    hashes: HashMap<Box<TargetPath>, u64>,
}

impl ImageLedger {
    #[context("creating a ledger of {recipe}")]
    pub(crate) fn new(recipe: &Recipe, image: &Image) -> anyhow::Result<ImageLedger> {
        let Image(image) = image;

        let mut ledger = ImageLedger::empty(recipe);

        for entry in WalkDir::new(image) {
            let entry = entry.context("walking the directory")?;

            let path =
                HostPath::new(entry.path()).expect("the items of WalkDir should be absolute");

            if entry.file_type().is_dir() {
                continue;
            }

            let file = File::open(path)?;
            let hash = rapidhash_v3_file(file)?;

            ledger
                .hashes
                .insert(TargetPath::from_path_and_root(path, image), hash);
        }

        Ok(ledger)
    }

    fn empty(recipe: &Recipe) -> ImageLedger {
        ImageLedger {
            name: Box::from(recipe.name()),
            version: recipe.version().clone(),
            hashes: HashMap::new(),
        }
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = (&TargetPath, u64)> {
        self.hashes.iter().map(|(file, hash)| (&**file, *hash))
    }
}
