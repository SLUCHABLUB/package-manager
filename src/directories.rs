use crate::State;
use crate::recipe::Download;
use crate::recipe::Recipe;
use fs_err::create_dir_all;
use fs_err::read_dir;
use fs_err::remove_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tracing::error;

// We use the `once_cell` crate since we need a `*try*` method which is not stable in std.
type OnceCell<T> = once_cell::unsync::OnceCell<T>;

macro_rules! concat_paths {
    ($($path:expr),*) => {
        ::std::iter::Iterator::collect::<::std::path::PathBuf>(
            <[&::std::path::Path; _]>::into_iter([
                $(
                    ::std::convert::AsRef::<::std::path::Path>::as_ref($path),
                )*
            ])
        )
    };
}

#[derive(Debug)]
pub struct RecipeDirectories<'state> {
    recipe: &'state Recipe,
    state: &'state State,

    /// The path to the (to be) built package tree.
    target: OnceCell<CacheDirectory>,
    /// The path to the working directory of the build.
    build_working: OnceCell<PathBuf>,
    /// The path to the root of the build if it differs from the source codes root.
    build_root: OnceCell<Option<PathBuf>>,
    /// The path to the source code.
    source: OnceCell<CacheDirectory>,
    /// The path to the bare repository (`.git` directory).
    repository: OnceCell<CacheDirectory>,
}

impl<'state> RecipeDirectories<'state> {
    // TODO: Take a locked recipe.
    pub(crate) fn new(
        recipe: &'state Recipe,
        state: &'state State,
    ) -> anyhow::Result<RecipeDirectories<'state>> {
        Ok(RecipeDirectories {
            recipe,
            state,

            target: OnceCell::new(),
            build_working: OnceCell::new(),
            build_root: OnceCell::new(),
            source: OnceCell::new(),
            repository: OnceCell::new(),
        })
    }

    pub(crate) fn target(&self) -> anyhow::Result<&CacheDirectory> {
        // TODO: Base this on the recipe hash.
        self.target.get_or_try_init(|| {
            CacheDirectory::new(concat_paths!(
                self.state.cache_directory(),
                "targets",
                &*self.recipe.name
            ))
        })
    }

    pub(crate) fn build_working(&self) -> anyhow::Result<&Path> {
        self.build_working
            .get_or_try_init(|| {
                let working =
                    concat_paths!(self.state.cache_directory(), "build", &*self.recipe.name);
                make_empty_directory(&working)?;
                Ok(working)
            })
            .map(|path| &**path)
    }

    pub(crate) fn build_root(&self) -> anyhow::Result<&Path> {
        let cached = self.build_root.get_or_try_init(|| {
            self.recipe
                .build
                .directory
                .as_ref()
                .map(|suffix| anyhow::Ok(self.source()?.path().join(suffix)))
                .transpose()
        })?;

        match cached {
            Some(path) => Ok(&**path),
            None => self.source().map(CacheDirectory::path),
        }
    }

    pub(crate) fn source(&self) -> anyhow::Result<&CacheDirectory> {
        self.source.get_or_try_init(|| {
            let mut path = self.state.cache_directory().join("sources");

            // TODO: Use the resolved url/version/commit.
            match &self.recipe.download {
                Download::Github {
                    repository,
                    version,
                } => {
                    path.push("github");
                    path.push(&**repository);
                    path.push(version.to_string());
                }
                Download::Tarball {
                    url,
                    compression: _,
                } => {
                    path.push("tarball");
                    path.push(&*urlencoding::encode(url.as_str()))
                }
                Download::TarballIndex {
                    url,
                    version,
                    filename_prefix: _,
                } => {
                    path.push("tarball_index");
                    path.push(&*urlencoding::encode(url.as_str()));
                    path.push(&*urlencoding::encode(&version.to_string()));
                }
            };

            CacheDirectory::new(path)
        })
    }

    pub(crate) fn repository(&self) -> anyhow::Result<&CacheDirectory> {
        self.repository.get_or_try_init(|| {
            let mut path = self.state.cache_directory().join("repositories");

            // TODO: Use the resolved git url.
            match &self.recipe.download {
                Download::Github {
                    repository,
                    version,
                } => {
                    path.push("github");
                    path.push(&**repository);
                    path.push(&*urlencoding::encode(&version.to_string()));
                }
                Download::Tarball { .. } | Download::TarballIndex { .. } => {
                    error!("internal error: non-git download requested a repository path");
                    path = PathBuf::from("/dev/null");
                }
            };

            CacheDirectory::new(path)
        })
    }
}

fn make_empty_directory(directory: &Path) -> anyhow::Result<()> {
    match remove_dir_all(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }?;
    create_dir_all(directory)?;

    Ok(())
}

fn is_directory_populated(directory: &Path) -> anyhow::Result<bool> {
    if !directory.is_dir() {
        return Ok(false);
    }

    Ok(read_dir(directory)?.find(Result::is_ok).is_some())
}

#[derive(Debug)]
pub(crate) struct CacheDirectory {
    path: PathBuf,
    is_populated: bool,
}

impl CacheDirectory {
    fn new(path: PathBuf) -> anyhow::Result<CacheDirectory> {
        let is_populated = is_directory_populated(&path)?;

        if !is_populated {
            make_empty_directory(&path)?;
        }

        Ok(CacheDirectory { path, is_populated })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn is_populated(&self) -> bool {
        self.is_populated
    }

    pub(crate) fn as_unpopulated(&self) -> Option<&Path> {
        (!self.is_populated).then_some(&self.path)
    }
}
