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
use crate::stage;
use anyhow::Context;
use fn_error_context::context;
use tracing::info;

struct LockedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
}

pub(crate) struct DownloadPlan {
    // TODO: Make this a DAG (for build dependencies).
    recipes: Vec<LockedRecipe>,
}

struct DownloadedRecipe {
    recipe: &'static Recipe,
    source: Source,
}

pub(crate) struct BuildPlan {
    recipes: Vec<DownloadedRecipe>,
}

struct BuiltRecipe {
    recipe: &'static Recipe,
    image: Image,
}

pub(crate) struct CheckPlan {
    recipes: Vec<BuiltRecipe>,
}

struct CheckedRecipe {
    recipe: &'static Recipe,
    image: Image,
    ledger: ImageLedger,
}

pub(crate) struct StagePlan {
    recipes: Vec<CheckedRecipe>,
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

    #[context("adding package `{name}` version {version} to the download plan")]
    pub(crate) fn add_package(
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
            self.add_package(dependency, version, recipes, host)?;
        }

        let locked = LockedRecipe {
            recipe,
            lock: recipe
                .download_data()
                .lock(host)
                .with_context(|| format!("locking {recipe}"))?,
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
        })
    }
}

impl StagePlan {
    pub(crate) fn stage(
        self,
        host: &HostDirectories,
        target: &TargetDirectories,
    ) -> anyhow::Result<SystemLedger> {
        let ledger = stage(
            self.recipes
                .into_iter()
                .map(|checked| (checked.recipe, checked.image, checked.ledger)),
            host,
            target,
        )?;

        Ok(ledger)
    }
}
