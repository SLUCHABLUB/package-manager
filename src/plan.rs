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
use crate::hash;
use crate::recipe::Build;
use crate::stage;
use anyhow::anyhow;
use by_address::ByAddress;
use daggy::Dag;
use daggy::NodeIndex;
use daggy::Walker as _;
use daggy::WouldCycle;
use fn_error_context::context;
use rapidhash::HashMapExt as _;
use rapidhash::RapidHashMap;
use std::collections::BTreeSet;
use std::mem;
use tracing::info;

#[derive(Debug)]
pub(crate) struct LockPlan {
    // TODO: Add a graph for the build dependencies.
    /// A graph containing all the recipes and edges for their runtime dependencies.
    dependency_graph: Dag<&'static Recipe, ()>,
    /// A map from recipe (assumed to be from the store) to index in the graph.
    recipe_indexes: RapidHashMap<ByAddress<&'static Recipe>, NodeIndex>,
}

#[derive(Debug)]
struct LockedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
}

#[derive(Debug)]
pub(crate) struct HashPlan {
    /// A graph containing all the recipes and edges for their runtime dependencies.
    dependency_graph: Dag<LockedRecipe, ()>,
}

#[derive(Debug)]
struct HashedRecipe {
    recipe: &'static Recipe,
    lock: DownloadLock,
    // TODO: Use this to find determine if the recipe is installed.
    #[expect(unused)]
    hash: u64,
}

#[derive(Debug)]
pub(crate) struct DownloadPlan {
    recipes: Vec<HashedRecipe>,
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

impl LockPlan {
    pub(crate) fn new() -> LockPlan {
        LockPlan {
            dependency_graph: Dag::new(),
            recipe_indexes: RapidHashMap::new(),
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

        if let Some(index) = self.recipe_indexes.get(&ByAddress(recipe)) {
            return Ok((*index, recipe));
        }

        let index = self.dependency_graph.add_node(recipe);
        self.recipe_indexes.insert(ByAddress(recipe), index);

        for (dependency, version) in &recipe.dependencies().versions {
            let (child_index, child) = self.add_recipe_inner(dependency, version, recipes)?;
            self.dependency_graph
                .add_edge(index, child_index, ())
                .map_err(|WouldCycle(())| {
                    anyhow!(
                        "cyclic dependency found between `{}` version `{}` and `{}` version `{}`",
                        recipe.name(),
                        recipe.version(),
                        child.name(),
                        child.version()
                    )
                })?;
        }

        Ok((index, recipe))
    }

    pub(crate) fn lock(self, host: &HostDirectories) -> anyhow::Result<HashPlan> {
        let LockPlan {
            dependency_graph: old_graph,
            recipe_indexes: old_indexs,
        } = self;

        let mut new_graph = Dag::new();
        let mut new_indexes = vec![NodeIndex::new(0); old_indexs.len()];

        info!("generating recipe download locks");

        for (ByAddress(recipe), old_index) in old_indexs {
            let locked = LockedRecipe {
                recipe,
                lock: find_cached_download_lock_or_create(recipe, host)?,
            };

            let new_index = new_graph.add_node(locked);
            new_indexes[old_index.index()] = new_index;
        }

        for old_index in 0..old_graph.node_count() {
            let old_index = NodeIndex::new(old_index);
            let new_index = new_indexes[old_index.index()];

            for (_edge_index, old_child_index) in old_graph.children(old_index).iter(&old_graph) {
                let new_child_index = new_indexes[old_child_index.index()];
                new_graph
                    .add_edge(new_index, new_child_index, ())
                    .expect("this should not cycle since the graph should be the same");
            }
        }

        info!("generated all locks");

        Ok(HashPlan {
            dependency_graph: new_graph,
        })
    }
}

impl HashPlan {
    pub(crate) fn hash(self) -> DownloadPlan {
        let HashPlan {
            mut dependency_graph,
        } = self;

        let mut hashed_recipes = Vec::with_capacity(dependency_graph.node_count());
        let mut hashes = vec![0; dependency_graph.node_count()].into_boxed_slice();

        info!("generating recipe hashes");

        for index in 0..dependency_graph.node_count() {
            let index = NodeIndex::new(index);

            let hash = Self::hash_recipe(index, &dependency_graph, &mut hashes);

            let LockedRecipe { recipe, lock } = dependency_graph
                .node_weight_mut(index)
                .expect("a node should exist for the index");

            let hashed_recipe = HashedRecipe {
                recipe,
                lock: mem::replace(lock, DownloadLock::None),
                hash,
            };

            hashed_recipes.push(hashed_recipe);
        }

        info!("generated all recipe hashes");

        DownloadPlan {
            recipes: hashed_recipes,
        }
    }

    fn hash_recipe(index: NodeIndex, graph: &Dag<LockedRecipe, ()>, hashes: &mut [u64]) -> u64 {
        // If a recipe actually has a hash of 0, it'll just be recalculated a couple of times.
        let cached = hashes[index.index()];
        if cached != 0 {
            return cached;
        }

        let mut dependencies = BTreeSet::new();

        for (_edge, child) in graph.children(index).iter(graph) {
            dependencies.insert(Self::hash_recipe(child, graph, hashes));
        }

        let node = graph
            .node_weight(index)
            .expect("a node should exist for the index");

        let hash_input: (&Build, &DownloadLock, BTreeSet<u64>) =
            (node.recipe.build_data(), &node.lock, dependencies);

        let hash = hash(hash_input);

        hashes[index.index()] = hash;
        hash
    }
}

impl DownloadPlan {
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
