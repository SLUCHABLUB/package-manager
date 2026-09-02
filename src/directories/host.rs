use crate::HostPath;
use crate::PACKAGE_NAME;
use crate::TargetDirectories;
use crate::directories::XDG_CACHE_HOME;
use anyhow::Context;
use const_str::join;
use std::path;

// TODO: Make this opaque.
#[derive(Debug)]
pub(crate) struct HostDirectories {
    pub(crate) download_locks: Box<HostPath>,
    pub(crate) repositories: Box<HostPath>,
    pub(crate) sources: Box<HostPath>,
    pub(crate) working: Box<HostPath>,
    pub(crate) images: Box<HostPath>,

    // TODO: This should depend on the install location/installation root.
    // TODO: Use RAII for this.
    pub(crate) staging: Box<HostPath>,

    // TODO: We should have locks on other things such as downloading, building and staging.
    pub(crate) lock_file: Box<HostPath>,
    pub(crate) journal_file: Box<HostPath>,
    /// The directory containing the journal file.
    pub(crate) journal_directory: Box<HostPath>,

    pub(crate) installation_root: Box<HostPath>,
}

impl HostDirectories {
    pub(crate) fn new(
        target: &TargetDirectories,
        installation_root: Box<HostPath>,
    ) -> anyhow::Result<HostDirectories> {
        Self::new_inner(target, installation_root).context("detecting the host directories")
    }

    fn new_inner(
        target: &TargetDirectories,
        installation_root: Box<HostPath>,
    ) -> Option<HostDirectories> {
        // TODO: Don't use XDG_DATA_HOME,
        // use a directory dependent on the target directories and installation root.

        let cache_directory = XDG_CACHE_HOME.as_ref()?;

        let data_directory = target.data().with_root(&installation_root);

        Some(HostDirectories {
            download_locks: cache_directory.with_suffix(join!(
                &[PACKAGE_NAME, "download-locks"],
                path::MAIN_SEPARATOR_STR
            )),
            repositories: cache_directory.with_suffix(join!(
                &[PACKAGE_NAME, "repositories"],
                path::MAIN_SEPARATOR_STR
            )),
            sources: cache_directory
                .with_suffix(join!(&[PACKAGE_NAME, "sources"], path::MAIN_SEPARATOR_STR)),
            working: cache_directory
                .with_suffix(join!(&[PACKAGE_NAME, "build"], path::MAIN_SEPARATOR_STR)),

            images: cache_directory
                .with_suffix(join!(&[PACKAGE_NAME, "images"], path::MAIN_SEPARATOR_STR)),

            staging: data_directory
                .with_suffix(join!(&[PACKAGE_NAME, "staging"], path::MAIN_SEPARATOR_STR)),
            lock_file: data_directory.with_suffix(join!(
                &[PACKAGE_NAME, "install-lock.toml"],
                path::MAIN_SEPARATOR_STR
            )),
            journal_file: data_directory.with_suffix(join!(
                &[PACKAGE_NAME, "install-journal.toml"],
                path::MAIN_SEPARATOR_STR
            )),
            journal_directory: data_directory.with_suffix(PACKAGE_NAME),

            installation_root,
        })
    }
}
