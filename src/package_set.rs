use crate::VersionRequirement;
use crate::manifest::Manifest;
use crate::prepare_install;
use crate::recipe::Recipe;
use directories::ProjectDirs;

/// A set of packages to be installed.
#[derive(Default)]
pub struct PackageSet<'recipes> {
    recipes: Vec<&'recipes Recipe>,
}

impl<'recipes> PackageSet<'recipes> {
    pub fn new() -> PackageSet<'recipes> {
        PackageSet::default()
    }

    fn contains(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.recipes
            .iter()
            .any(|recipe| recipe.provides(package_name, version))
    }

    pub fn add(
        &mut self,
        package_name: &str,
        version: &VersionRequirement,
        manifest: &'recipes Manifest,
    ) -> anyhow::Result<()> {
        if self.contains(package_name, version) {
            return Ok(());
        }

        let recipe = manifest.find_recipe(package_name, version)?;

        for (dependency, version) in &recipe.dependencies.versions {
            self.add(dependency, version, manifest)?;
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
