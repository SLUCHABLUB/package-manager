use crate::HostPath;
use crate::PACKAGE_NAME;
use crate::directories::XDG_CACHE_HOME;
use crate::directories::XDG_CONFIG_HOME;
use crate::directories::XDG_DATA_HOME;
use anyhow::Context;
use const_str::concat;
use std::path::MAIN_SEPARATOR;

#[derive(Debug)]
pub(crate) struct HostDirectories {
    pub(crate) user_configuration: Box<HostPath>,

    pub(crate) repositories: Box<HostPath>,
    pub(crate) sources: Box<HostPath>,
    pub(crate) working: Box<HostPath>,
    pub(crate) targets: Box<HostPath>,

    pub(crate) staging: Box<HostPath>,
}

impl HostDirectories {
    pub(crate) fn new() -> anyhow::Result<HostDirectories> {
        Self::new_inner().context("detecting the host directories")
    }

    fn new_inner() -> Option<HostDirectories> {
        Some(HostDirectories {
            targets: XDG_CACHE_HOME.as_ref()?.with_suffix(concat!(
                PACKAGE_NAME,
                MAIN_SEPARATOR,
                "targets"
            )),
            working: XDG_CACHE_HOME.as_ref()?.with_suffix(concat!(
                PACKAGE_NAME,
                MAIN_SEPARATOR,
                "build"
            )),
            sources: XDG_CACHE_HOME.as_ref()?.with_suffix(concat!(
                PACKAGE_NAME,
                MAIN_SEPARATOR,
                "sources"
            )),
            repositories: XDG_CACHE_HOME.as_ref()?.with_suffix(concat!(
                PACKAGE_NAME,
                MAIN_SEPARATOR,
                "repositories"
            )),
            user_configuration: XDG_CONFIG_HOME.as_ref()?.with_suffix(PACKAGE_NAME),
            staging: XDG_DATA_HOME.as_ref()?.with_suffix(PACKAGE_NAME),
        })
    }
}
