use crate::HostPath;
use crate::TargetPath;
use crate::directories::XDG_BIN_HOME;
use crate::directories::XDG_CONFIG_HOME;
use crate::directories::XDG_DATA_HOME;
use crate::directories::XDG_INCLUDE_HOME;
use crate::directories::XDG_LIB_HOME;
use crate::directories::XDG_PREFIX_HOME;
use crate::directories::XDG_RUNTIME_DIR;
use crate::directories::XDG_STATE_HOME;
use anyhow::Context as _;
use once_cell::sync::Lazy as LazyLock;
use std::path::Path;

pub(crate) struct TargetDirectories {
    prefix: &'static TargetPath,

    configuration: &'static TargetPath,
    data: &'static TargetPath,
    executables: &'static TargetPath,
    headers: &'static TargetPath,
    internal_executables: &'static TargetPath,
    libraries: &'static TargetPath,
    runtime: &'static TargetPath,
    state: &'static TargetPath,
    system_executables: &'static TargetPath,
}

impl TargetDirectories {
    pub(crate) fn user() -> anyhow::Result<TargetDirectories> {
        Self::user_inner().context("detecting the target directories")
    }

    fn user_inner() -> Option<TargetDirectories> {
        type EnvironmenVariable = &'static LazyLock<Option<Box<HostPath>>>;

        let from_environment =
            |variable: EnvironmenVariable| variable.as_deref().map(HostPath::to_target_path);

        let prefix = from_environment(&XDG_PREFIX_HOME)?;

        let configuration = from_environment(&XDG_CONFIG_HOME)?;
        let data = from_environment(&XDG_DATA_HOME)?;
        let executables = from_environment(&XDG_BIN_HOME)?;
        let headers = from_environment(&XDG_INCLUDE_HOME)?;
        let libraries = from_environment(&XDG_LIB_HOME)?;
        let state = from_environment(&XDG_STATE_HOME)?;

        let runtime = from_environment(&XDG_RUNTIME_DIR)?;

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

    pub(crate) fn fhs_usr() -> TargetDirectories {
        let target_path = |path| TargetPath::new(Path::new(path)).expect("path should be absolute");

        TargetDirectories {
            prefix: target_path("/usr"),
            configuration: target_path("/etc"),
            data: target_path("/usr/share"),
            executables: target_path("/usr/bin"),
            headers: target_path("/usr/include"),
            internal_executables: target_path("/usr/libexec"),
            libraries: target_path("/usr/lib"),
            runtime: target_path("/run"),
            state: target_path("/var/lib"),
            system_executables: target_path("/usr/sbin"),
        }
    }

    pub(crate) fn prefix(&self) -> &TargetPath {
        self.prefix
    }

    pub(crate) fn configuration(&self) -> &TargetPath {
        self.configuration
    }

    pub(crate) fn data(&self) -> &TargetPath {
        self.data
    }

    pub(crate) fn executables(&self) -> &TargetPath {
        self.executables
    }

    pub(crate) fn headers(&self) -> &TargetPath {
        self.headers
    }

    pub(crate) fn internal_executables(&self) -> &TargetPath {
        self.internal_executables
    }

    pub(crate) fn libraries(&self) -> &TargetPath {
        self.libraries
    }

    pub(crate) fn runtime(&self) -> &TargetPath {
        self.runtime
    }

    pub(crate) fn state(&self) -> &TargetPath {
        self.state
    }

    pub(crate) fn system_executables(&self) -> &TargetPath {
        self.system_executables
    }
}
