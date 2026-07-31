use crate::DownloadLock;
use crate::HostDirectories;
use crate::Image;
use crate::ImageLedger;
use crate::Recipe;
use crate::RecipeStore;
use crate::Source;
use crate::SystemLedger;
use crate::TargetDirectories;
use crate::VersionRequirement;
use crate::build;
use crate::check_image;
use crate::download;
use crate::find_cached_download_lock_or_create;
use crate::stage;
use fn_error_context::context;
use tracing::info;

#[derive(Debug)]
struct LockedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
}

#[derive(Debug)]
pub(crate) struct DownloadPlan {
    // TODO: Make this a DAG (for build dependencies).
    recipes: Vec<LockedRecipe>,
}

#[derive(Debug)]
struct DownloadedRecipe {
    recipe: &'static Recipe,
    source: Source,
}

#[derive(Debug)]
struct InstalledRecipe {
    ledger: ImageLedger,
}

#[derive(Debug)]
pub(crate) struct BuildPlan {
    recipes: Vec<DownloadedRecipe>,
    installed_recipes: Vec<InstalledRecipe>,
}

#[derive(Debug)]
struct BuiltRecipe {
    recipe: &'static Recipe,
    image: Image,
}

#[derive(Debug)]
pub(crate) struct CheckPlan {
    recipes: Vec<BuiltRecipe>,
    installed_recipes: Vec<InstalledRecipe>,
}

#[derive(Debug)]
struct CheckedRecipe {
    recipe: &'static Recipe,
    image: Image,
    ledger: ImageLedger,
}

#[derive(Debug)]
pub(crate) struct StagePlan {
    recipes: Vec<CheckedRecipe>,
    installed_recipes: Vec<InstalledRecipe>,
}

impl DownloadPlan {
    pub(crate) fn new() -> DownloadPlan {
        DownloadPlan {
            recipes: Vec::new(),
        }
    }

    fn contains(&self, package_name: &str, version_requirement: &VersionRequirement) -> bool {
        self.recipes.iter().any(|locked| {
            locked.recipe.name() == package_name
                && locked.recipe.version().satisfies(version_requirement)
        })
    }

    #[context("adding a recipe for `{name}` version {version} to the download plan")]
    pub(crate) fn add_recipe(
        &mut self,
        name: &str,
        version: &VersionRequirement,
        recipes: &'static RecipeStore,
        host: &HostDirectories,
    ) -> anyhow::Result<()> {
        if self.contains(name, version) {
            return Ok(());
        }

        let recipe = recipes.find(name, version)?;

        for (dependency, version) in &recipe.dependencies().versions {
            self.add_recipe(dependency, version, recipes, host)?;
        }

        let locked = LockedRecipe {
            recipe,
            lock: find_cached_download_lock_or_create(recipe, host)?,
        };

        self.recipes.push(locked);

        Ok(())
    }

    pub(crate) fn download(self, host: &HostDirectories) -> anyhow::Result<BuildPlan> {
        let mut downloaded_recipes = Vec::new();

        info!("downloading the recipe sources");

        // TODO: Parallelise.
        for locked in self.recipes {
            let source = download(locked.recipe, &locked.lock, host)?;

            downloaded_recipes.push(DownloadedRecipe {
                recipe: locked.recipe,
                source,
            });
        }

        info!("finished all downloads");

        Ok(BuildPlan {
            recipes: downloaded_recipes,
            // TODO: Look at what is installed (and compare the hashes).
            installed_recipes: Vec::new(),
        })
    }
}

impl BuildPlan {
    pub(crate) fn build(
        self,
        target: &TargetDirectories,
        host: &HostDirectories,
    ) -> anyhow::Result<CheckPlan> {
        let mut built_recipes = Vec::new();

        info!("building the packages");

        // TODO: Parallelise.
        for downloaded in self.recipes {
            let image = build(downloaded.recipe, downloaded.source, target, host)?;

            built_recipes.push(BuiltRecipe {
                recipe: downloaded.recipe,
                image,
            });
        }

        info!("built all packages");

        Ok(CheckPlan {
            recipes: built_recipes,
            installed_recipes: self.installed_recipes,
        })
    }
}

impl CheckPlan {
    pub(crate) fn check(self) -> anyhow::Result<StagePlan> {
        let mut checked_recipes = Vec::new();

        info!("checking and inventorying all package images");

        // TODO: Parallelise.
        for built in self.recipes {
            let ledger = check_image(built.recipe, &built.image)?;

            checked_recipes.push(CheckedRecipe {
                recipe: built.recipe,
                image: built.image,
                ledger,
            });
        }

        info!("all checks complete");

        Ok(StagePlan {
            recipes: checked_recipes,
            installed_recipes: self.installed_recipes,
        })
    }
}

impl StagePlan {
    pub(crate) fn stage(
        self,
        host: &HostDirectories,
        target: &TargetDirectories,
    ) -> anyhow::Result<SystemLedger> {
        let mut ledger = stage(
            self.recipes
                .into_iter()
                .map(|checked| (checked.recipe, checked.image, checked.ledger)),
            host,
            target,
        )?;

        for installed in self.installed_recipes {
            ledger.add_image(installed.ledger);
        }

        Ok(ledger)
    }
}
