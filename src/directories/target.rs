use crate::TargetPath;
use crate::directories::HOME;
use crate::directories::PREFIX;
use crate::directories::XDG_BIN_HOME;
use crate::directories::XDG_CONFIG_HOME;
use crate::directories::XDG_DATA_HOME;
use crate::directories::XDG_INCLUDE_HOME;
use crate::directories::XDG_LIB_HOME;
use crate::directories::XDG_RUNTIME_DIR;
use crate::directories::XDG_STATE_HOME;
use anyhow::Context as _;

// TODO: Make this opaque.
pub(crate) struct TargetDirectories {
    pub prefix: Box<TargetPath>,

    pub configuration: &'static TargetPath,
    pub data: &'static TargetPath,
    pub executables: &'static TargetPath,
    pub headers: &'static TargetPath,
    pub internal_executables: &'static TargetPath,
    pub libraries: &'static TargetPath,
    pub runtime: &'static TargetPath,
    pub state: &'static TargetPath,
    pub system_executables: &'static TargetPath,
}

impl TargetDirectories {
    pub(crate) fn user() -> anyhow::Result<TargetDirectories> {
        Self::user_inner().context("detecting the target directories")
    }

    fn user_inner() -> Option<TargetDirectories> {
        // TODO: Should we add a "XDG_PREFIX_HOME" variable?
        let prefix = HOME.as_ref()?.with_suffix(PREFIX).into_target_path();

        let configuration = XDG_CONFIG_HOME.as_ref()?.to_target_path();
        let data = XDG_DATA_HOME.as_ref()?.to_target_path();
        let executables = XDG_BIN_HOME.as_ref()?.to_target_path();
        let headers = XDG_INCLUDE_HOME.as_ref()?.to_target_path();
        let libraries = XDG_LIB_HOME.as_ref()?.to_target_path();
        let state = XDG_STATE_HOME.as_ref()?.to_target_path();

        let runtime = XDG_RUNTIME_DIR.as_ref()?.to_target_path();

        Some(TargetDirectories {
            prefix,
            configuration,
            data,
            executables,
            headers,
            internal_executables: libraries,
            libraries,
            runtime,
            state,
            system_executables: executables,
        })
    }
}
