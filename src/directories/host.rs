use crate::HostPath;
use crate::PACKAGE_NAME;
use crate::directories::XDG_CACHE_HOME;
use crate::directories::XDG_CONFIG_HOME;
use crate::directories::XDG_DATA_HOME;
use anyhow::Context;
use const_str::join;
use std::path;

// TODO: Make this opaque.
#[derive(Debug)]
pub(crate) struct HostDirectories {
    pub(crate) user_configuration: Box<HostPath>,

    pub(crate) repositories: Box<HostPath>,
    pub(crate) sources: Box<HostPath>,
    pub(crate) working: Box<HostPath>,
    pub(crate) targets: Box<HostPath>,

    pub(crate) staging: Box<HostPath>,

    // TODO: We should have locks on other things such as downloading, building and staging.
    pub(crate) lock_file: Box<HostPath>,
    pub(crate) journal_file: Box<HostPath>,
    /// The directory containing the journal file.
    pub(crate) journal_directory: Box<HostPath>,

    // TODO: This should be based on the install location.
    pub(crate) ledger_file: Box<HostPath>,
}

impl HostDirectories {
    pub(crate) fn new() -> anyhow::Result<HostDirectories> {
        Self::new_inner().context("detecting the host directories")
    }

    fn new_inner() -> Option<HostDirectories> {
        Some(HostDirectories {
            user_configuration: XDG_CONFIG_HOME.as_ref()?.with_suffix(PACKAGE_NAME),

            repositories: XDG_CACHE_HOME.as_ref()?.with_suffix(join!(
                &[PACKAGE_NAME, "repositories"],
                path::MAIN_SEPARATOR_STR
            )),
            sources: XDG_CACHE_HOME
                .as_ref()?
                .with_suffix(join!(&[PACKAGE_NAME, "sources"], path::MAIN_SEPARATOR_STR)),
            working: XDG_CACHE_HOME
                .as_ref()?
                .with_suffix(join!(&[PACKAGE_NAME, "build"], path::MAIN_SEPARATOR_STR)),
            targets: XDG_CACHE_HOME
                .as_ref()?
                .with_suffix(join!(&[PACKAGE_NAME, "targets"], path::MAIN_SEPARATOR_STR)),

            staging: XDG_DATA_HOME
                .as_ref()?
                .with_suffix(join!(&[PACKAGE_NAME, "staging"], path::MAIN_SEPARATOR_STR)),

            lock_file: XDG_DATA_HOME.as_ref()?.with_suffix(join!(
                &[PACKAGE_NAME, "install-lock.toml"],
                path::MAIN_SEPARATOR_STR
            )),
            journal_file: XDG_DATA_HOME.as_ref()?.with_suffix(join!(
                &[PACKAGE_NAME, "install-journal.toml"],
                path::MAIN_SEPARATOR_STR
            )),
            journal_directory: XDG_DATA_HOME.as_ref()?.with_suffix(PACKAGE_NAME),
            ledger_file: XDG_DATA_HOME.as_ref()?.with_suffix(join!(
                &[PACKAGE_NAME, "ledger.toml"],
                path::MAIN_SEPARATOR_STR
            )),
        })
    }
}
