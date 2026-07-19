use anyhow::Context;
use once_cell::sync::OnceCell as OnceLock;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

// TODO: Make this opaque.
pub(crate) struct Directories {
    pub prefix: PathBuf,

    pub configuration: PathBuf,
    pub data: PathBuf,
    pub executables: PathBuf,
    pub headers: PathBuf,
    pub internal_executables: PathBuf,
    pub libraries: PathBuf,
    pub runtime: PathBuf,
    pub state: PathBuf,
}

impl Directories {
    pub(crate) fn user() -> anyhow::Result<&'static Directories> {
        static USER: OnceLock<Directories> = OnceLock::new();

        USER.get_or_try_init(|| {
            let home = env::home_dir().context("detecting the home directory")?;

            // TODO: Set the variables on esoteric OSes like darwin and dos.

            // TODO: Should we add a "XDG_PREFIX_HOME" variable?
            let prefix = home.join(".local");

            let configuration = env::var_os("XDG_DATA_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| home.join(".config"));

            let data = env::var_os("XDG_DATA_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("share"));

            let executables = env::var_os("XDG_BIN_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("bin"));

            let headers = env::var_os("XDG_INCLUDE_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("include"));

            let libraries = env::var_os("XDG_LIB_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("lib"));

            // TODO: This should be under `/run`,
            // but there does not seem to be a standardised subdirectory.
            // On my current machine it would be `/run/user/{uid}`.
            let runtime = prefix.join("run");

            let state = env::var_os("XDG_STATE_HOME")
                .and_then(parse_xdg_directory)
                .unwrap_or_else(|| prefix.join("state"));

            Ok(Directories {
                prefix,
                executables,
                data,
                internal_executables: libraries.clone(),
                state,
                configuration,
                headers,
                libraries,
                runtime,
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
