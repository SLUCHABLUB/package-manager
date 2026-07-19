use anyhow::Context;
use once_cell::sync::OnceCell as OnceLock;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

// TODO: Make this opaque.
pub(crate) struct Directories {
    pub prefix: PathBuf,
    pub executables: PathBuf,
}

impl Directories {
    pub(crate) fn user() -> anyhow::Result<&'static Directories> {
        static USER: OnceLock<Directories> = OnceLock::new();

        USER.get_or_try_init(|| {
            let home = env::home_dir().context("detecting the home directory")?;

            // TODO: Set the variables on esoteric OSes like darwin and dos.

            // TODO: Should we add a "XDG_PREFIX_HOME" variable?
            let prefix = home.join(".local");

            let executables = env::var_os("XDG_BIN_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("bin"));

            Ok(Directories {
                prefix,
                executables,
            })
        })
    }
}

fn parse_xdg_directory(path: OsString) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }

    let path = PathBuf::from(path);

    if !path.is_absolute() {
        return None;
    }

    Some(path)
}
