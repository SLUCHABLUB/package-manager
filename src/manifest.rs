use crate::PackageSet;
use crate::ResultExtension as _;
use crate::VersionRequirement;
use crate::recipe::Recipe;
use anyhow::Context as _;
use anyhow::bail;
use fs_err::read_to_string;
use serde::Deserialize;
use serde::Serialize;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::Path;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    packages: HashMap<Box<str>, VersionRequirement>,
    /// A map from package name to recipe name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    providers: HashMap<Box<str>, Box<str>>,
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    recipe_directories: Box<[Box<Path>]>,

    #[serde(skip)]
    recipes: OnceCell<Box<[Recipe]>>,
}

impl Manifest {
    pub fn package_set(&self) -> anyhow::Result<PackageSet<'_>> {
        let mut package_set = PackageSet::new();

        for (package, version) in &self.packages {
            package_set.add(package, version, self)?;
        }

        Ok(package_set)
    }

    // TODO: Cache the recipes.
    pub fn find_recipe(
        &self,
        package_name: &str,
        version: &VersionRequirement,
    ) -> anyhow::Result<&Recipe> {
        if let Some(recipe_name) = self.providers.get(package_name) {
            return self.find_recipe_named(recipe_name);
        }

        let mut recipes = self
            .recipes()
            .filter(|recipe| recipe.provides(package_name, version));

        let Some(recipe) = recipes.next() else {
            bail!("no recipe provides `{package_name}` version {version}");
        };

        if recipes.next().is_some() {
            bail!(
                "multiple recipes provide `{package_name}` version {version}, please select a provider in the manifest"
            );
        }

        Ok(recipe)
    }

    fn find_recipe_named(&self, name: &str) -> anyhow::Result<&Recipe> {
        let mut recipes = self.recipes().filter(|recipe| &*recipe.name == name);

        let Some(recipe) = recipes.next() else {
            bail!("no recipe named `{name}`");
        };

        if recipes.next().is_some() {
            bail!("multiple recipes are named `{name}`");
        }

        Ok(recipe)
    }

    fn recipes(&self) -> impl Iterator<Item = &Recipe> {
        self.recipes
            .get_or_init(|| {
                self.recipe_directories
                    .iter()
                    .filter_map(|directory| {
                        Some(read_dir(directory).ok_or_log()?.filter_map(|entry| {
                            let entry = entry.ok_or_log()?;
                            let path = entry.path();

                            if entry.file_type().ok_or_log()?.is_dir() {
                                warn!("skipping the directory {}", path.display());
                                return None;
                            }

                            let recipe = read_to_string(&path).ok_or_log()?;
                            let recipe = toml::from_str::<Recipe>(&recipe)
                                .with_context(|| format!("parsing `{}`", path.display()))
                                .ok_or_log()?;

                            Some(recipe)
                        }))
                    })
                    .flatten()
                    .collect()
            })
            .iter()
    }
}
