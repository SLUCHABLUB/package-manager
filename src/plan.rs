use crate::Dag;
use crate::Dependencies;
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
use crate::build::Sandbox;
use crate::check_image;
use crate::download;
use crate::find_cached_download_lock_or_create;
use crate::hash;
use crate::recipe::Build;
use crate::stage;
use anyhow::anyhow;
use fn_error_context::context;
use petgraph::acyclic::AcyclicEdgeError;
use petgraph::graph::NodeIndex;
use std::collections::BTreeSet;
use std::ptr;
use tracing::info;

#[derive(Debug)]
struct RuntimeDependency;
#[derive(Copy, Clone, Debug)]
struct BuildDependency;

// The two graphs share nodes && indexes.
#[derive(Debug)]
pub(crate) struct LockPlan {
    // TODO: Can these be merged into a single graph?
    /// A graph containing all the recipes and edges for their runtime dependencies.
    runtime_dependencies: Dag<&'static Recipe, RuntimeDependency>,

    /// A graph containing all the recipes and edges for their runtime dependencies.
    build_dependencies: Dag<&'static Recipe, BuildDependency>,
}

#[derive(Debug)]
struct LockedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
}

#[derive(Debug)]
pub(crate) struct HashPlan {
    /// A graph containing all the recipes and edges for their runtime dependencies.
    runtime_dependencies: Dag<LockedRecipe, RuntimeDependency>,
    build_dependencies: Dag<&'static Recipe, BuildDependency>,
}

#[derive(Debug)]
struct HashedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
    hash: u64,
}

#[derive(Debug)]
pub(crate) struct DownloadPlan {
    recipes: Dag<HashedRecipe, BuildDependency>,
}

#[derive(Debug)]
struct DownloadedRecipe {
    recipe: &'static Recipe,
    source: Source,
}

#[derive(Debug)]
struct InstalledRecipe {
    // TODO: We need to reconstruct the image before marking the recipe as installed
    // since the files can be modified.
    ledger: ImageLedger,
}

#[derive(Debug)]
pub(crate) struct BuildPlan {
    recipes: Dag<DownloadedRecipe, BuildDependency>,
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

impl LockPlan {
    pub(crate) fn new() -> LockPlan {
        LockPlan {
            runtime_dependencies: Dag::new(),
            build_dependencies: Dag::new(),
        }
    }

    pub(crate) fn add_recipe(
        &mut self,
        name: &str,
        version: &VersionRequirement,
        recipes: &'static RecipeStore,
    ) -> anyhow::Result<()> {
        self.add_recipe_inner(name, version, recipes)?;
        Ok(())
    }

    #[context("adding a recipe for `{name}` version {version} to the plan")]
    fn add_recipe_inner(
        &mut self,
        name: &str,
        version: &VersionRequirement,
        recipes: &'static RecipeStore,
    ) -> anyhow::Result<(NodeIndex, &'static Recipe)> {
        let recipe = recipes.find(name, version)?;

        if let Some(recipe) = self.find_added_recipe(recipe) {
            return Ok(recipe);
        }

        let index = self.runtime_dependencies.add_node(recipe);

        // This should return the same value as `index`.
        self.build_dependencies.add_node(recipe);

        self.add_children(
            recipe,
            index,
            recipe.dependencies(),
            |plan| &mut plan.runtime_dependencies,
            || RuntimeDependency,
            "runtime",
            recipes,
        )?;

        self.add_children(
            recipe,
            index,
            &recipe.build_data().dependencies,
            |plan| &mut plan.build_dependencies,
            || BuildDependency,
            "build",
            recipes,
        )?;

        Ok((index, recipe))
    }

    #[allow(clippy::too_many_arguments)]
    fn add_children<Dependency: 'static>(
        &mut self,
        recipe: &'static Recipe,
        index: NodeIndex,
        dependencies: &Dependencies,
        // Since `&mut self` takes all of self as mutable, this needs to be a function.
        mut graph: impl FnMut(&mut Self) -> &mut Dag<&'static Recipe, Dependency>,
        mut edge: impl FnMut() -> Dependency,
        dependency_type: &'static str,
        recipes: &'static RecipeStore,
    ) -> anyhow::Result<()> {
        for (dependency, version) in &dependencies.versions {
            let (child_index, child) = self.add_recipe_inner(dependency, version, recipes)?;
            graph(self)
                .add_edge(index, child_index, edge())
                .map_err(|error| runtime_cycle_error(&error, recipe, child, dependency_type))?;
        }

        Ok(())
    }

    fn find_added_recipe(&self, recipe: &'static Recipe) -> Option<(NodeIndex, &'static Recipe)> {
        // Since both graphs have the same nodes and indexes, it doesn't matter which one we use here.
        self.runtime_dependencies
            .indexed_nodes()
            .map(|(index, node)| (index, *node))
            .find(|(_, node)| ptr::eq(*node, recipe))
    }

    pub(crate) fn lock(self, host: &HostDirectories) -> anyhow::Result<HashPlan> {
        let LockPlan {
            runtime_dependencies,
            build_dependencies,
        } = self;

        info!("generating recipe download locks");

        let runtime_dependencies = runtime_dependencies.try_map(|recipe| {
            anyhow::Ok(LockedRecipe {
                recipe,
                lock: find_cached_download_lock_or_create(recipe, host)?,
            })
        })?;

        info!("generated all locks");

        Ok(HashPlan {
            runtime_dependencies,
            build_dependencies,
        })
    }
}

fn runtime_cycle_error(
    error: &AcyclicEdgeError<NodeIndex>,
    parent: &Recipe,
    child: &Recipe,
    dependency_type: &'static str,
) -> anyhow::Error {
    match error {
        // The index here is either the parent or the child.
        AcyclicEdgeError::Cycle(_) => anyhow!(
            "cyclic {dependency_type} dependency found between `{}` version `{}` and `{}` version `{}`",
            parent.name(),
            parent.version(),
            child.name(),
            child.version()
        ),
        AcyclicEdgeError::SelfLoop => anyhow!(
            "{dependency_type} dependency found from `{}` version `{}` to itself",
            parent.name(),
            parent.version(),
        ),
        AcyclicEdgeError::InvalidEdge => anyhow!(
            "unable to add {dependency_type} dependency from `{}` version `{}` to `{}` version `{}`",
            parent.name(),
            parent.version(),
            child.name(),
            child.version()
        ),
    }
}

impl HashPlan {
    pub(crate) fn hash(self) -> DownloadPlan {
        let HashPlan {
            runtime_dependencies,
            build_dependencies,
        } = self;

        info!("generating recipe hashes");

        let mut locked_recipes: Vec<_> = runtime_dependencies.into_nodes().collect();
        let mut locked_recipe = |recipe| {
            let index = locked_recipes
                .iter()
                .position(|locked| ptr::eq(locked.recipe, recipe))
                .expect("the recipe should not have been taken");

            locked_recipes.swap_remove(index)
        };

        let hashed_recipes = build_dependencies.recursive_map(|recipe, dependencies| {
            let locked_recipe = locked_recipe(recipe);

            let hash = hash_recipe(&locked_recipe, dependencies);

            let LockedRecipe { recipe, lock } = locked_recipe;

            HashedRecipe { recipe, lock, hash }
        });

        info!("generated all recipe hashes");

        DownloadPlan {
            recipes: hashed_recipes,
        }
    }
}

fn hash_recipe<'recipes>(
    locked: &LockedRecipe,
    dependencies: impl IntoIterator<Item = &'recipes HashedRecipe>,
) -> u64 {
    let dependencies = dependencies
        .into_iter()
        .map(|dependency| dependency.hash)
        .collect();

    let hash_input: (&Build, &DownloadLock, BTreeSet<u64>) =
        (locked.recipe.build_data(), &locked.lock, dependencies);

    hash(hash_input)
}

impl DownloadPlan {
    pub(crate) fn download(self, host: &HostDirectories) -> anyhow::Result<BuildPlan> {
        let locked_recipes = self.recipes;

        info!("downloading the recipe sources");

        let downloaded_recipes = locked_recipes.try_map(|locked| {
            let source = download(locked.recipe, &locked.lock, host)?;

            anyhow::Ok(DownloadedRecipe {
                recipe: locked.recipe,
                source,
            })
        })?;

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
        sandbox: Sandbox,
        target: &TargetDirectories,
        host: &HostDirectories,
    ) -> anyhow::Result<CheckPlan> {
        let downloaded_recipes = self.recipes;

        info!("building the packages");

        // TODO: Parallelise.
        let built_recipes = downloaded_recipes
            .try_reduce_leaves::<BuiltRecipe, _>(|downloaded, children| {
                // TODO: Make the child images available for the build command

                let image = build(
                    downloaded.recipe,
                    downloaded.source,
                    children.map(|built| &built.image),
                    sandbox,
                    target,
                    host,
                )?;

                anyhow::Ok(BuiltRecipe {
                    recipe: downloaded.recipe,
                    image,
                })
            })?
            .collect();

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
