use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use fn_error_context::context;
use fs_err::read;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Recipe {
    #[serde(skip)]
    pub name: Box<str>,

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
    #[context("parsing the recipe at `{}`", path.display())]
    pub(crate) fn read_from(path: &Path) -> anyhow::Result<Recipe> {
        let file_name = path
            .file_name()
            .context("determining the recipe's file name")?;
        let file_name = file_name.to_string_lossy();
        let file_name = file_name.strip_suffix(".toml").unwrap_or(&file_name);

        let bytes = read(path)?;
        let mut recipe: Recipe = toml::from_slice(&bytes)?;

        recipe.name = Box::from(file_name);

        Ok(recipe)
    }

    pub(crate) fn provides(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.provides
            .get(package_name)
            .is_some_and(|provided_version| provided_version.satisfies(version))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Download {
    Github {
        repository: Box<str>,
        version: VersionRequirement,
    },
    Tarball {
        url: Url,
        compression: Option<Compression>,
    },
    TarballIndex {
        url: Url,
        version: VersionRequirement,
        file_name_prefix: Box<str>,
    },
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Compression {
    None,
    Xz,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Build {
    #[serde(default)]
    pub dependencies: Dependencies,
    pub directory: Option<Box<Path>>,

    // The keys and values should be `OsStr`s but those serialise weirdly.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment_variables: HashMap<Box<str>, Box<str>>,
    #[serde(flatten)]
    pub system: BuildSystem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "system", rename_all = "snake_case")]
pub(crate) enum BuildSystem {
    Cargo {
        // TODO: locked: bool,
        // TODO: profile: Box<str>,
        // TODO: no-default-features: bool,
        // TODO: bins/examples
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        features: Box<[Box<str>]>,
        target: Option<Box<str>>,
    },
    ConfigureMake {
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        configure_flags: Box<[Box<str>]>,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Install {
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
pub(crate) struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, VersionRequirement>,
}
