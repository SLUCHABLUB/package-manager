use crate::Version;
use crate::find_recipe;
use crate::prepare_install;
use crate::recipe::Recipe;
use directories::ProjectDirs;

/// A set of packages to be installed.
#[derive(Default)]
pub struct PackageSet {
    recipes: Vec<Recipe>,
}

impl PackageSet {
    pub fn new() -> PackageSet {
        PackageSet::default()
    }

    fn contains(&self, package_name: &str, version: &Version) -> bool {
        self.recipes
            .iter()
            .any(|recipe| recipe.provides(package_name, version))
    }

    pub fn add(&mut self, package_name: &str, version: &Version) -> anyhow::Result<()> {
        if self.contains(package_name, version) {
            return Ok(());
        }

        let recipe = find_recipe(package_name, version)?;

        for (dependency, version) in &recipe.dependencies.versions {
            self.add(dependency, version)?;
        }

        self.recipes.push(recipe);

        Ok(())
    }

    pub fn prepare_install(&self, project_directories: &ProjectDirs) -> anyhow::Result<()> {
        for recipe in &self.recipes {
            prepare_install(recipe, project_directories)?;
        }

        Ok(())
    }
}
