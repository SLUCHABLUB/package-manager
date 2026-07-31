use crate::Dependencies;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Hash, Debug, Serialize, Deserialize)]
pub(crate) struct Build {
    #[serde(default)]
    pub dependencies: Dependencies,
    // TODO: Enforce that this is relative.
    pub directory: Option<Box<Path>>,

    // TODO: The keys and values should be `OsStr`s but those serialise weirdly.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<Box<str>, Box<str>>,
    #[serde(flatten)]
    pub system: BuildSystem,
}

#[derive(Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "system", rename_all = "snake_case")]
pub(crate) enum BuildSystem {
    None,
    Cargo {
        // TODO: locked: bool,
        // TODO: profile: Box<str>,
        // TODO: no-default-features: bool,
        // TODO: toolchain: Box<str>,
        // TODO: bins/examples

        // TODO: Allow the user to specify multiple binaries or examples.
        binary: Box<str>,

        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        features: Box<[Box<str>]>,
        target: Option<Box<str>>,
    },
    ConfigureMake {
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        configure_flags: Box<[Box<str>]>,
    },
}
