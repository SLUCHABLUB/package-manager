use crate::BuildPlan;
use crate::HostDirectories;
use crate::Manifest;
use crate::Recipe;
use crate::SystemLedger;
use crate::TargetDirectories;
use crate::VersionRequirement;
use crate::install;
use anyhow::bail;
use anyhow::ensure;
use fn_error_context::context;
use fs_err::remove_dir_all;
use once_cell::unsync::OnceCell;
use std::io;

#[derive(Debug)]
pub struct State {
    main_manifest: Manifest,
    directories: HostDirectories,
    recipes: OnceCell<Box<[Recipe]>>,
}

impl State {
    #[context("initialising the package manager state")]
    pub fn initialise() -> anyhow::Result<State> {
        let directories = HostDirectories::new()?;

        let manifest =
            Manifest::read_from(directories.user_configuration.with_suffix("manifest.toml"))?;

        Ok(State {
            main_manifest: manifest,
            recipes: OnceCell::new(),
            directories,
        })
    }

    /// Downloads, builds, and installs all packages.
    pub fn install(&self, into: &TargetDirectories) -> anyhow::Result<()> {
        let ledger = self.stage(into)?;

        install(&self.directories, &ledger, into)?;

        Ok(())
    }

    /// Downloads, builds and stages all packages.
    pub(crate) fn stage(&self, into: &TargetDirectories) -> anyhow::Result<SystemLedger> {
        match remove_dir_all(&self.directories.staging) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            result @ Err(_) => result?,
        }

        self.build_plan()?.stage(into)
    }

    pub(crate) fn directories(&self) -> &HostDirectories {
        &self.directories
    }

    fn recipes(&self) -> impl Iterator<Item = &Recipe> {
        self.recipes
            .get_or_init(|| self.main_manifest.read_recipes().collect())
            .iter()
    }

    pub(crate) fn recipe_for_package(
        &self,
        name: &str,
        version: &VersionRequirement,
    ) -> anyhow::Result<&Recipe> {
        if let Some(recipe_name) = self.main_manifest.provider(name) {
            let recipe = self.recipe_named(recipe_name)?;

            ensure!(
                recipe.provides(name, version),
                "the specified provider for the `{name}` package does not provide version {version}"
            );

            return Ok(recipe);
        }

        let mut recipes = self
            .recipes()
            .filter(|recipe| recipe.provides(name, version));

        let Some(recipe) = recipes.next() else {
            bail!("no recipe provides `{name}` version {version}");
        };

        if recipes.next().is_some() {
            bail!(
                "multiple recipes provide `{name}` version {version}, please select a provider in the manifest"
            );
        }

        Ok(recipe)
    }

    fn recipe_named(&self, name: &str) -> anyhow::Result<&Recipe> {
        let mut recipes = self.recipes().filter(|recipe| &*recipe.name == name);

        let Some(recipe) = recipes.next() else {
            bail!("no recipe named `{name}`");
        };

        if recipes.next().is_some() {
            bail!("multiple recipes are named `{name}`");
        }

        Ok(recipe)
    }

    #[context("creating a build plan")]
    fn build_plan(&self) -> anyhow::Result<BuildPlan<'_>> {
        let mut plan = BuildPlan::new(self);

        for (package, version) in self.main_manifest.packages() {
            plan.add_package(package, version)?;
        }

        Ok(plan)
    }
}
