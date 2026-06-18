use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub name: Box<str>,
    pub author: Box<str>,

    pub download: Download,
    pub build: Build,
    #[serde(default)]
    pub install: Install,
    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Download {
    Github { repository: Box<str> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    dependencies: Dependencies,
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
    /// Install the package into `$HOME` using the `$XDG_*_DIRS`.
    User,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, Version>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Version {
    Semver(semver::VersionReq),
    Opaque { opaque: Box<str> },
}
