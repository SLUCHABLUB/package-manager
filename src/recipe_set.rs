use crate::VersionRequirement;
use crate::manifest::Manifest;
use crate::prepare_install;
use crate::recipe::Recipe;
use directories::ProjectDirs;

/// A set of packages to be installed.
#[derive(Default)]
pub struct RecipeSet<'recipes> {
    recipes: Vec<&'recipes Recipe>,
}

impl<'recipes> RecipeSet<'recipes> {
    pub fn new() -> RecipeSet<'recipes> {
        RecipeSet::default()
    }

    fn contains(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.recipes
            .iter()
            .any(|recipe| recipe.provides(package_name, version))
    }

    pub fn add_package(
        &mut self,
        name: &str,
        version: &VersionRequirement,
        manifest: &'recipes Manifest,
    ) -> anyhow::Result<()> {
        if self.contains(name, version) {
            return Ok(());
        }

        let recipe = manifest.find_recipe(name, version)?;

        for (dependency, version) in &recipe.dependencies.versions {
            self.add_package(dependency, version, manifest)?;
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
