mod host;
mod target;

use crate::HostPath;
use const_str::concat;
use once_cell::sync::Lazy as LazyLock;
use std::env;
use std::path::MAIN_SEPARATOR;
use std::path::PathBuf;
use tracing::error;
use tracing::warn;

pub(crate) use host::HostDirectories;

pub use target::TargetDirectories;

const PREFIX: &str = ".local";

static HOME: LazyLock<Option<Box<HostPath>>> = LazyLock::new(|| {
    let Some(buffer) = env::home_dir() else {
        error!("could not detect the home directory");
        return None;
    };

    // I think this should be a no-op.
    let path = buffer.into_boxed_path();

    match HostPath::new_boxed(path) {
        Ok(path) => Some(path),
        Err(path) => {
            error!("the home directory (`{}`) is not absolute", path.display());
            None
        }
    }
});

// TODO: Set the XDG variables on esoteric OSes like darwin and dos.

static XDG_CACHE_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable("XDG_CACHE_HOME", concat!(".cache")));

static XDG_CONFIG_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable("XDG_CONFIG_HOME", concat!(".config")));

static XDG_DATA_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable("XDG_DATA_HOME", concat!(PREFIX, MAIN_SEPARATOR, "share")));

static XDG_BIN_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable("XDG_BIN_HOME", concat!(PREFIX, MAIN_SEPARATOR, "bin")));

static XDG_INCLUDE_HOME: LazyLock<Option<Box<HostPath>>> = LazyLock::new(|| {
    parse_xdg_variable(
        "XDG_INCLUDE_HOME",
        concat!(PREFIX, MAIN_SEPARATOR, "include"),
    )
});

static XDG_LIB_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable("XDG_LIB_HOME", concat!(PREFIX, MAIN_SEPARATOR, "lib")));

static XDG_STATE_HOME: LazyLock<Option<Box<HostPath>>> = LazyLock::new(|| {
    parse_xdg_variable("XDG_STATE_HOME", concat!(PREFIX, MAIN_SEPARATOR, "state"))
});

static XDG_RUNTIME_DIR: LazyLock<Option<Box<HostPath>>> = LazyLock::new(|| {
    parse_xdg_variable_or_else("XDG_RUNTIME_DIR", || {
        warn!("XDG_RUNTIME_DIR was not set, falling back to `$XDG_STATE_HOME/run`");
        XDG_STATE_HOME.as_ref().map(|path| path.with_suffix("run"))
    })
});

fn parse_xdg_variable(variable: &'static str, fallback: &'static str) -> Option<Box<HostPath>> {
    parse_xdg_variable_or_else(variable, || {
        HOME.as_ref().map(|home| home.with_suffix(fallback))
    })
}

fn parse_xdg_variable_or_else(
    variable: &'static str,
    fallback: impl FnOnce() -> Option<Box<HostPath>>,
) -> Option<Box<HostPath>> {
    let Some(path) = env::var_os(variable) else {
        return fallback();
    };

    if path.is_empty() {
        warn!("the `{variable}` environment variable was set but empty");
        return None;
    }

    // This *should* be a no-op.
    let path = PathBuf::from(path).into_boxed_path();

    match HostPath::new_boxed(path) {
        Ok(path) => Some(path),
        Err(path) => {
            warn!(
                "the contents of the `{variable}` environment variable (`{}`) was not an absolute path",
                path.display()
            );
            None
        }
    }
}
