use crate::HostPath;
use crate::LockPlan;
use crate::Recipe;
use crate::TargetDirectories;
use crate::VersionRequirement;
use crate::recipe_store::RecipeStore;
use crate::result::convert_result;
use anyhow::Context;
use anyhow::ensure;
use fs_err::DirEntry;
use fs_err::read_dir;
use fs_err::read_to_string;
use itertools::Itertools as _;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct Manifest {
    path: Box<HostPath>,
    parent_directory: Box<HostPath>,

    data: ManifestData,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestData {
    install_location: InstallLocation,
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    recipe_directories: Box<[Box<Path>]>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    packages: HashMap<Box<str>, VersionRequirement>,
}

impl Manifest {
    pub(crate) fn read_from(path: Box<HostPath>) -> anyhow::Result<Manifest> {
        let manifest = read_to_string(&path)?;
        let data: ManifestData = toml::from_str(&manifest)?;

        let parent_directory = path
            .parent()
            .with_context(|| format!("getting the parent of `{path}`"))?
            .into();

        Ok(Manifest {
            path,
            parent_directory,
            data,
        })
    }

    fn read_recipes(&self) -> impl Iterator<Item = anyhow::Result<Recipe>> {
        self.data
            .recipe_directories
            .iter()
            .map(|relative| self.resolve_relative_path(relative))
            .map(read_recipes_from_directory)
            .flatten_ok()
            .map(Result::flatten)
    }

    fn resolve_relative_path(&self, path: &Path) -> Box<HostPath> {
        self.parent_directory.with_suffix(path)
    }

    pub(crate) fn create_recipe_store(&self) -> anyhow::Result<RecipeStore> {
        let recipes = self.read_recipes().collect::<anyhow::Result<_>>()?;

        Ok(RecipeStore::from_recipes(recipes))
    }

    pub(crate) fn packages(&self) -> impl Iterator<Item = (&str, &VersionRequirement)> {
        self.data
            .packages
            .iter()
            .map(|(package, version)| (&**package, version))
    }

    pub(crate) fn lock_plan(&self, recipes: &'static RecipeStore) -> anyhow::Result<LockPlan> {
        let mut plan = LockPlan::new();

        for (name, version) in self.packages() {
            plan.add_recipe(name, version, recipes)?;
        }

        Ok(plan)
    }

    pub(crate) fn target_directories(&self) -> anyhow::Result<TargetDirectories> {
        match self.data.install_location {
            InstallLocation::FileHierarchySystemUniversalSystemResources => {
                Ok(TargetDirectories::fhs_usr())
            }
            InstallLocation::User => TargetDirectories::user(),
        }
    }
}

#[expect(clippy::needless_pass_by_value, clippy::boxed_local)]
fn read_recipes_from_directory(
    directory: Box<HostPath>,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Recipe>> + use<>> {
    Ok(read_dir(&*directory)?
        .map(convert_result)
        .map_ok(read_recipe_from_directory_entry)
        .map(Result::flatten))
}

#[expect(clippy::needless_pass_by_value)]
fn read_recipe_from_directory_entry(entry: DirEntry) -> anyhow::Result<Recipe> {
    let path = entry.path();
    let path = HostPath::new(&path).expect("readdir output should be absolute for absolute input");

    ensure!(!path.is_dir(), "expected a file, found a directory: {path}");

    Recipe::read_from(path)
}

impl Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the manifest at `{}`", self.path)
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum InstallLocation {
    #[serde(rename = "/usr")]
    FileHierarchySystemUniversalSystemResources,
    #[serde(rename = "/home")]
    User,
}
