use crate::DownloadLock;
use crate::HostPath;
use crate::Recipe;
use crate::State;
use crate::serde::once_cell_as_option;
use anyhow::Context;
use fn_error_context::context;
use fs_err::create_dir_all;
use fs_err::read_dir;
use fs_err::remove_dir_all;
use once_cell::unsync::OnceCell;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::fmt;
use std::fmt::Debug;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher as _;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tracing::warn;
use url::Url;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct RecipeDirectories {
    /// The path to the (to be) built package tree.
    #[serde(with = "once_cell_as_option")]
    target: OnceCell<CacheDirectory>,
    /// The path to the working directory of the build.
    #[serde(with = "once_cell_as_option")]
    build_working: OnceCell<Box<HostPath>>,
    /// The path to the root of the build if it differs from the source codes root.
    #[serde(with = "once_cell_as_option")]
    build_root: OnceCell<Option<Box<HostPath>>>,
    /// The path to the source code.
    #[serde(with = "once_cell_as_option")]
    source: OnceCell<CacheDirectory>,
    /// The path to the bare repository (`.git` directory).
    #[serde(with = "once_cell_as_option")]
    repository: OnceCell<CacheDirectory>,
}

impl RecipeDirectories {
    pub(crate) fn target(&self, recipe: &Recipe, state: &State) -> anyhow::Result<&CacheDirectory> {
        // TODO: Base this on the recipe hash.
        self.target.get_or_try_init(|| {
            CacheDirectory::new(
                state
                    .cache_directory()
                    .with_suffix("targets")
                    .with_suffix(&*recipe.name),
            )
        })
    }

    #[context("preparing the working directory for the build")]
    pub(crate) fn build_working(
        &self,
        recipe: &Recipe,
        state: &State,
    ) -> anyhow::Result<&HostPath> {
        self.build_working
            .get_or_try_init(|| {
                // TODO: Put the cache subdirectories in the state struct.
                let working = state
                    .cache_directory()
                    .with_suffix("build")
                    .with_suffix(&*recipe.name);
                make_empty_directory(&*working)?;
                Ok(working)
            })
            .map(|path| &**path)
    }

    pub(crate) fn build_root(&self, recipe: &Recipe, state: &State) -> anyhow::Result<&HostPath> {
        let cached = self.build_root.get_or_try_init(|| {
            recipe
                .build
                .directory
                .as_ref()
                .map(|suffix| {
                    anyhow::Ok(
                        self.source(recipe.download_lock(state)?, state)?
                            .path()
                            .with_suffix(suffix),
                    )
                })
                .transpose()
        })?;

        match cached {
            Some(path) => Ok(&**path),
            None => self
                .source(recipe.download_lock(state)?, state)
                .map(CacheDirectory::path),
        }
    }

    pub(crate) fn source(
        &self,
        lock: &DownloadLock,
        state: &State,
    ) -> anyhow::Result<&CacheDirectory> {
        self.source.get_or_try_init(|| {
            let mut buffer = PathBuf::from(state.cache_directory());
            buffer.push("sources");

            match lock {
                DownloadLock::None => {
                    return Ok(CacheDirectory::empty());
                }
                DownloadLock::Git { url, commit } => {
                    buffer.push(encode_url(url));
                    buffer.push(commit.to_string());
                }
                DownloadLock::Tarball {
                    url,
                    compression: _,
                } => {
                    buffer.push(encode_url(url));
                }
            }

            // Since we initialise the buffer with an absolute path, it should remain absolute.
            CacheDirectory::new(
                HostPath::new_boxed(buffer.into_boxed_path()).expect("the path should be absolute"),
            )
        })
    }

    pub(crate) fn repository_from_git_url(
        &self,
        url: &Url,
        state: &State,
    ) -> anyhow::Result<&CacheDirectory> {
        self.repository.get_or_try_init(|| {
            CacheDirectory::new(
                state
                    .cache_directory()
                    .with_suffix("repositories")
                    .with_suffix(encode_url(url)),
            )
        })
    }
}

fn encode_url(url: &Url) -> String {
    let human_readable_prefix = url
        .path_segments()
        .and_then(Iterator::last)
        .or_else(|| url.domain())
        .unwrap_or_else(|| {
            warn!("could not retrieve a human readable component from the `{url}` url");
            "weird-url"
        });

    let mut hasher = DefaultHasher::new();
    url.as_str().hash(&mut hasher);

    let injection_factor = hasher.finish();

    // TODO: Add an extension?
    format!("{human_readable_prefix}-{injection_factor}")
}

fn make_empty_directory(directory: impl AsRef<Path>) -> anyhow::Result<()> {
    let directory = directory.as_ref();

    match remove_dir_all(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }?;
    create_dir_all(directory)?;

    Ok(())
}

fn is_directory_populated(directory: impl AsRef<Path>) -> anyhow::Result<bool> {
    let directory = directory.as_ref();

    if !directory.is_dir() {
        return Ok(false);
    }

    Ok(read_dir(directory)?.find(Result::is_ok).is_some())
}

#[derive(Debug)]
pub(crate) struct CacheDirectory {
    path: Box<HostPath>,
    is_populated: OnceTrue,
}

impl CacheDirectory {
    fn new(path: Box<HostPath>) -> anyhow::Result<CacheDirectory> {
        let is_populated =
            is_directory_populated(&*path).context("detecting if the cache is populated")?;

        if !is_populated {
            make_empty_directory(&*path).context("preparing the directory")?;
        }

        Ok(CacheDirectory {
            path,
            is_populated: OnceTrue::new(is_populated),
        })
    }

    fn empty() -> CacheDirectory {
        CacheDirectory {
            path: HostPath::new_boxed(Box::from(Path::new("/var/empty")))
                .expect("`/var/empty` should be an absolute path"),
            is_populated: OnceTrue::new(true),
        }
    }

    pub(crate) fn path(&self) -> &HostPath {
        &self.path
    }

    pub(crate) fn as_populated(&self) -> Option<&HostPath> {
        self.is_populated.get().then_some(&self.path)
    }

    // What a name.
    pub(crate) fn as_populated_then_run_or_populate_with<E>(
        &self,
        on_populated: impl FnOnce(&HostPath),
        populate: impl FnOnce(&HostPath) -> Result<(), E>,
    ) -> Result<(), E> {
        self.as_populated_then_try_or_populate_with(
            |path| {
                on_populated(path);
                Ok(())
            },
            populate,
        )
    }

    // What a name.
    pub(crate) fn as_populated_then_try_or_populate_with<T, E>(
        &self,
        on_populated: impl FnOnce(&HostPath) -> Result<T, E>,
        populate: impl FnOnce(&HostPath) -> Result<T, E>,
    ) -> Result<T, E> {
        // We need to so this really ugly solution since the compiler cannot prove that both of our functions will run and that they won't run simultaneously.
        let return_value: OnceCell<Result<T, E>> = OnceCell::new();

        self.is_populated.then_run_or_set_if(
            || {
                return_value
                    .set(on_populated(&self.path))
                    .ok()
                    .expect("only one branch should be taken");
            },
            || {
                let result = populate(&self.path);
                let success = result.is_ok();
                return_value
                    .set(result)
                    .ok()
                    .expect("only one branch should be taken");
                success
            },
        );

        return_value
            .into_inner()
            .expect("one branch should be taken")
    }
}

/// Serialises only the path.
impl Serialize for CacheDirectory {
    fn serialize<S>(&self, serialiser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.path.serialize(serialiser)
    }
}

/// Deserialises from only a path and re-detects the cache.
impl<'data> Deserialize<'data> for CacheDirectory {
    fn deserialize<D>(deserialiser: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'data>,
    {
        CacheDirectory::new(Box::<HostPath>::deserialize(deserialiser)?)
            .map_err(serde::de::Error::custom)
    }
}

struct True;

#[derive(Default)]
pub(crate) struct OnceTrue {
    /// If it is set, the value is true otherwise it is false.
    cell: OnceCell<True>,
}

impl OnceTrue {
    pub(crate) fn new(set: bool) -> OnceTrue {
        let cell = OnceTrue::default();
        if set {
            cell.set();
        }
        cell
    }

    pub(crate) fn get(&self) -> bool {
        self.cell.get().is_some()
    }

    pub(crate) fn set(&self) {
        let _ = self.cell.set(True);
    }

    pub(crate) fn then_run_or_set_if(
        &self,
        on_true: impl FnOnce(),
        on_false: impl FnOnce() -> bool,
    ) {
        let mut was_true = true;

        let _ = self.cell.get_or_try_init(|| {
            was_true = false;
            let should_set = on_false();
            should_set.then_some(True).ok_or(())
        });

        if was_true {
            on_true();
        }
    }
}

impl Debug for OnceTrue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.get())
    }
}
