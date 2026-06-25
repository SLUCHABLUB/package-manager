use crate::Version;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub name: Box<str>,
    pub author: Box<str>,
    pub version: Version,

    #[serde(default)]
    pub provides: HashMap<Box<str>, Version>,

    pub download: Download,
    pub build: Build,
    #[serde(default)]
    pub install: Install,
    #[serde(default)]
    pub dependencies: Dependencies,
}

impl Recipe {
    pub fn provides(&self, package_name: &str, version: &Version) -> bool {
        &*self.name == package_name && self.version.satisfies(version)
            || self
                .provides
                .get(package_name)
                .is_some_and(|provided_version| provided_version.satisfies(version))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Download {
    Github { repository: Box<str> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub dependencies: Dependencies,
    #[serde(flatten)]
    pub system: BuildSystem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "system", rename_all = "snake_case")]
pub enum BuildSystem {
    Cargo {
        // TODO: Make this a common field.
        // The keys and variables should be `OsStr`s but those serialise weirdly.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        environment_variables: HashMap<Box<str>, Box<str>>,
        // TODO: locked: bool,
        // TODO: profile: Box<str>,
        // TODO: no-default-features: bool,
        // TODO: bins/examples
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        features: Box<[Box<str>]>,
        target: Option<Box<str>>,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Install {
    /// Install the package into `/usr/local`.
    Local,
    /// Install the package into `/opt`.
    Opt,
    /// Install the package into `/usr`.
    #[default]
    System,
    /// Install the package into `$HOME` using the `$XDG_*_HOME`.
    User,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, Version>,
}
