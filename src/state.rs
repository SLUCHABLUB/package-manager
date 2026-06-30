use crate::BuildPlan;
use crate::Manifest;
use crate::PACKAGE_NAME;
use crate::VersionRequirement;
use crate::recipe::Recipe;
use anyhow::Context as _;
use anyhow::bail;
use directories::ProjectDirs;
use fs_err::read_to_string;
use std::cell::OnceCell;
use std::path::Path;
use std::path::PathBuf;

pub struct State {
    main_manifest: Manifest,
    directories: ProjectDirs,
    recipes: OnceCell<Box<[Recipe]>>,
}

impl State {
    pub fn initialise(manifest: &Path) -> anyhow::Result<State> {
        let manifest = read_to_string(manifest)?;
        let manifest = toml::from_str(&manifest)?;

        let directories = ProjectDirs::from_path(PathBuf::from(PACKAGE_NAME))
            .context("determining project directories")?;

        Ok(State {
            main_manifest: manifest,
            recipes: OnceCell::new(),
            directories,
        })
    }

    pub fn prepare_to_install(&self) -> anyhow::Result<()> {
        self.build_plan()?.prepare_to_install()
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
