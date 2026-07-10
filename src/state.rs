use crate::BuildPlan;
use crate::Manifest;
use crate::PACKAGE_NAME;
use crate::Recipe;
use crate::VersionRequirement;
use anyhow::Context as _;
use anyhow::bail;
use directories::ProjectDirs;
use fn_error_context::context;
use fs_err::remove_dir_all;
use once_cell::unsync::OnceCell;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub struct State {
    main_manifest: Manifest,
    directories: ProjectDirs,
    recipes: OnceCell<Box<[Recipe]>>,
}

impl State {
    #[context("initialising the package manager state")]
    pub fn initialise() -> anyhow::Result<State> {
        let directories = ProjectDirs::from_path(PathBuf::from(PACKAGE_NAME))
            .context("determining project directories")?;

        let manifest = Manifest::read_from(directories.config_dir().join("manifest.toml"))?;

        Ok(State {
            main_manifest: manifest,
            recipes: OnceCell::new(),
            directories,
        })
    }

    pub fn stage(&self) -> anyhow::Result<()> {
        // TODO: Base this on the install location.
        let staging = self.directories.data_dir().join("staging");

        match remove_dir_all(&staging) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            result @ Err(_) => result?,
        }

        self.build_plan()?.stage(&staging)
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
        if let Some(recipe_name) = self.main_manifest.providers.get(name) {
            return self.recipe_named(recipe_name);
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

        for (package, version) in &self.main_manifest.packages {
            plan.add_package(package, version)?;
        }

        Ok(plan)
    }

    pub(crate) fn cache_directory(&self) -> &Path {
        self.directories.cache_dir()
    }
}
