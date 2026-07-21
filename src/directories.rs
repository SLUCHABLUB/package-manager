use crate::TargetPath;
use anyhow::Context;
use anyhow::anyhow;
use once_cell::sync::OnceCell as OnceLock;
use std::env;
use std::path::PathBuf;
use tracing::warn;

// TODO: Make this opaque.
pub(crate) struct Directories {
    pub prefix: Box<TargetPath>,

    pub configuration: Box<TargetPath>,
    pub data: Box<TargetPath>,
    pub executables: Box<TargetPath>,
    pub headers: Box<TargetPath>,
    // TODO: Make this optional?
    pub internal_executables: Box<TargetPath>,
    pub libraries: Box<TargetPath>,
    pub runtime: Box<TargetPath>,
    pub state: Box<TargetPath>,
}

impl Directories {
    pub(crate) fn user() -> anyhow::Result<&'static Directories> {
        static USER: OnceLock<Directories> = OnceLock::new();

        USER.get_or_try_init(|| {
            let home = env::home_dir().context("detecting the home directory")?;
            let home = TargetPath::new_boxed(home.into_boxed_path()).map_err(|home| {
                anyhow!("the home directory (`{}`) is not absolute", home.display())
            })?;

            // TODO: Set the variables on esoteric OSes like darwin and dos.

            // TODO: Should we add a "XDG_PREFIX_HOME" variable?
            let prefix = home.with_suffix(".local");

            let configuration =
                parse_xdg_variable("XDG_DATA_HOME").unwrap_or_else(|| home.with_suffix(".config"));

            let data =
                parse_xdg_variable("XDG_DATA_HOME").unwrap_or_else(|| prefix.with_suffix("share"));

            let executables =
                parse_xdg_variable("XDG_BIN_HOME").unwrap_or_else(|| prefix.with_suffix("bin"));

            let headers = parse_xdg_variable("XDG_INCLUDE_HOME")
                .unwrap_or_else(|| prefix.with_suffix("include"));

            let libraries =
                parse_xdg_variable("XDG_LIB_HOME").unwrap_or_else(|| prefix.with_suffix("lib"));

            // TODO: Yes there is, it's `XDG_RUNTIME_DIR`!
            // TODO: This should be under `/run`,
            // but there does not seem to be a standardised subdirectory.
            // On my current machine it would be `/run/user/{uid}`.
            let runtime = prefix.with_suffix("run");

            let state =
                parse_xdg_variable("XDG_STATE_HOME").unwrap_or_else(|| prefix.with_suffix("state"));

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

fn parse_xdg_variable(variable: &'static str) -> Option<Box<TargetPath>> {
    let path = env::var_os(variable)?;

    if path.is_empty() {
        warn!("the `{variable}` environment variable was set but empty");
        return None;
    }

    let path = PathBuf::from(path);

    if !path.is_absolute() {
        warn!("the contents of the `{variable}` environment variable was not an absolute path");
        return None;
    }

    // This *should* be a no-op.
    let path = path.into_boxed_path();

    Some(TargetPath::new_boxed(path).expect("the path should be absolute"))
}
