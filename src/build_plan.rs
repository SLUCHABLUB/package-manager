use crate::Recipe;
use crate::State;
use crate::VersionRequirement;
use crate::prepare_to_install;

/// A set of recipes to be installed.
pub(crate) struct BuildPlan<'state> {
    state: &'state State,
    recipes: Vec<&'state Recipe>,
}

impl<'state> BuildPlan<'state> {
    pub(crate) fn new(state: &'state State) -> BuildPlan<'state> {
        BuildPlan {
            state,
            recipes: Vec::new(),
        }
    }

    fn contains(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.recipes
            .iter()
            .any(|recipe| recipe.provides(package_name, version))
    }

    pub(crate) fn add_package(
        &mut self,
        name: &str,
        version: &VersionRequirement,
    ) -> anyhow::Result<()> {
        if self.contains(name, version) {
            return Ok(());
        }

        let recipe = self.state.recipe_for_package(name, version)?;

        for (dependency, version) in &recipe.dependencies.versions {
            self.add_package(dependency, version)?;
        }

        self.recipes.push(recipe);

        Ok(())
    }

    pub(crate) fn prepare_to_install(&self) -> anyhow::Result<()> {
        // TODO: Parallelise.
        for recipe in &self.recipes {
            prepare_to_install(recipe, self.state)?;
        }

        Ok(())
    }
}
