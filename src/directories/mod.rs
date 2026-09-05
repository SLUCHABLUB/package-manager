mod host;
mod target;

use crate::HostPath;
use once_cell::sync::Lazy as LazyLock;
use std::env;
use std::path::PathBuf;
use tracing::error;
use tracing::warn;

pub(crate) use host::HostDirectories;
pub(crate) use target::TargetDirectories;

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

static XDG_PREFIX_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_home("XDG_PREFIX_HOME", ".local"));

static XDG_CACHE_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_home("XDG_CACHE_HOME", ".cache"));

static XDG_CONFIG_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_home("XDG_CONFIG_HOME", ".config"));

static XDG_DATA_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_prefix("XDG_DATA_HOME", "share"));

static XDG_BIN_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_prefix("XDG_BIN_HOME", "bin"));

static XDG_INCLUDE_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_prefix("XDG_INCLUDE_HOME", "include"));

static XDG_LIB_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_prefix("XDG_LIB_HOME", "lib"));

static XDG_STATE_HOME: LazyLock<Option<Box<HostPath>>> =
    LazyLock::new(|| parse_xdg_variable_or_prefix("XDG_STATE_HOME", "state"));

static XDG_RUNTIME_DIR: LazyLock<Option<Box<HostPath>>> = LazyLock::new(|| {
    parse_xdg_variable_or_else("XDG_RUNTIME_DIR", || {
        warn!("XDG_RUNTIME_DIR was not set, falling back to `$XDG_STATE_HOME/run`");
        XDG_STATE_HOME.as_ref().map(|path| path.with_suffix("run"))
    })
});

fn parse_xdg_variable_or_home(
    variable: &'static str,
    fallback: &'static str,
) -> Option<Box<HostPath>> {
    parse_xdg_variable_or_else(variable, || {
        HOME.as_ref().map(|home| home.with_suffix(fallback))
    })
}

fn parse_xdg_variable_or_prefix(
    variable: &'static str,
    fallback: &'static str,
) -> Option<Box<HostPath>> {
    parse_xdg_variable_or_else(variable, || {
        XDG_PREFIX_HOME
            .as_ref()
            .map(|prefix| prefix.with_suffix(fallback))
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
