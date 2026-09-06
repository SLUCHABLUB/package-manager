mod lock;

use crate::HostDirectories;
use crate::HostPath;
use crate::PACKAGE_NAME;
use crate::ResultExtension;
use crate::SystemLedger;
use crate::TargetDirectories;
use crate::TargetPath;
use crate::install::lock::lock;
use crate::install::lock::unlock;
use anyhow::Context as _;
use anyhow::bail;
use const_str::concat;
use fn_error_context::context;
use fs_err as fs;
use fs_err::File;
use fs_err::create_dir_all;
use fs_err::remove_file;
use rapidhash::v3::rapidhash_v3_file;
use serde::Serialize;
use std::io::Write;
use tracing::info;
use tracing::warn;

const TEMPORARY_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "temporary");
const BACKUP_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "backup");

// TODO: Take an installation method parameter.
pub(crate) fn install(
    ledger: SystemLedger,
    host: &HostDirectories,
    target: &TargetDirectories,
) -> anyhow::Result<()> {
    let lock = lock(host)?;

    let old_ledger = SystemLedger::read_from_host(target, host)?;

    warn!("installing... don't touch the file system please");

    let mut journal = Journal::from_ledgers(ledger, &old_ledger, host)?;

    journal.write_to_system()?;

    // TODO: Henceforth, if we fail, we can try to recover using the journal.

    create_temporary_files(&journal, host)?;

    create_backups(&journal, host)?;

    switch_to_temporary_files(&journal, host)?;

    info!("installation complete; cleaning up");

    let install_operations = journal.remove_from_system(host)?;

    remove_backups(&install_operations, host);

    // If this fails, the kernel will release the lock anyway.
    unlock(lock)?;

    info!("cleaning complete; you may touch the file system");

    Ok(())
}

enum ConflictCheckResult {
    /// The file did not exist in the last generation.
    New,
    /// The file exited in the last generation and has not been modified.
    Updated,
    /// The file existed on the system but was not managed by the package manager.
    Unmanaged,
    /// The file is managed but was modified since the last generation.
    Modified,
    /// The file was the same in the last generation.
    RemainsSame,
}

fn check_conflict(
    file: &TargetPath,
    new_hash: u64,
    old_ledger: &SystemLedger,
    root: &HostPath,
) -> anyhow::Result<ConflictCheckResult> {
    let host_path = file.with_root(root);

    Ok(if host_path.exists() {
        if let Some(old_hash) = old_ledger.hash(file) {
            let existing_file = File::open(&**host_path)?;
            let existing_hash = rapidhash_v3_file(existing_file)?;

            if old_hash == existing_hash {
                if old_hash == new_hash {
                    ConflictCheckResult::RemainsSame
                } else {
                    ConflictCheckResult::Updated
                }
            } else {
                ConflictCheckResult::Modified
            }
        } else {
            ConflictCheckResult::Unmanaged
        }
    } else {
        ConflictCheckResult::New
    })
}

fn handle_conflict(
    conflict: &ConflictCheckResult,
    file: &TargetPath,
    journal: &mut Journal,
) -> anyhow::Result<()> {
    match conflict {
        ConflictCheckResult::New => {
            journal.operations.push(InstallOperation {
                file: Box::from(file),
                temporary: file.with_extension(TEMPORARY_EXTENSION),
                backup: None,
            });
        }
        ConflictCheckResult::Updated => {
            journal.operations.push(InstallOperation {
                file: Box::from(file),
                temporary: file.with_extension(TEMPORARY_EXTENSION),
                backup: Some(file.with_extension(BACKUP_EXTENSION)),
            });
        }
        ConflictCheckResult::Unmanaged => {
            // TODO: We could prompt the user here.
            bail!("the unmanaged file at `{file}` would be overwritten");
        }
        ConflictCheckResult::Modified => {
            // TODO: We could prompt the user here.
            bail!("the file at `{file}` has been modified since the last installation");
        }
        ConflictCheckResult::RemainsSame => {
            // No update needed.
        }
    }

    Ok(())
}

fn ledger_install(ledger: &SystemLedger, root: &HostPath) -> InstallOperation {
    let should_backup = ledger.path().with_root(root).exists();

    InstallOperation {
        file: Box::from(ledger.path()),
        temporary: ledger.path().with_extension(TEMPORARY_EXTENSION),
        backup: should_backup.then(|| ledger.path().with_extension(BACKUP_EXTENSION)),
    }
}

#[derive(Serialize)]
struct Journal {
    operations: Vec<InstallOperation>,

    // These are here to keep the files open until we destroy the journal.
    #[serde(skip_serializing)]
    file: File,
    #[serde(skip_serializing)]
    directory: File,
}

impl Journal {
    fn open(host: &HostDirectories) -> anyhow::Result<Journal> {
        // TODO: Handle the case where the file exists (recover the installation).

        let directory = File::open(&*host.journal_directory)?;
        let file = File::create_new(&*host.journal_file)?;

        Ok(Journal {
            operations: Vec::new(),
            file,
            directory,
        })
    }

    fn from_ledgers(
        ledger: SystemLedger,
        old_ledger: &SystemLedger,
        host: &HostDirectories,
    ) -> anyhow::Result<Self> {
        // TODO: Handle the case where the file exists (recover the installation).

        let mut journal = Journal::open(host)?;

        for (recipe, file, hash) in ledger.files() {
            let conflict = check_conflict(file, hash, old_ledger, &host.installation_root)?;
            handle_conflict(&conflict, file, &mut journal)
                .with_context(|| format!("conflict when installing {recipe}"))?;
        }

        journal
            .operations
            .push(ledger_install(&ledger, &host.installation_root));

        drop(ledger);

        Ok(journal)
    }

    fn write_to_system(&mut self) -> anyhow::Result<()> {
        let serialised_journal = self.serialise()?;

        self.file.write_all(serialised_journal.as_ref())?;

        // If we cannot ensure that the journal is on the system,
        // bailing is the best option.
        self.file.sync_all()?;

        // We need to sync the directory as well so the directory entries are updated.
        self.directory.sync_all()?;

        Ok(())
    }

    fn serialise(&self) -> anyhow::Result<impl AsRef<[u8]> + use<>> {
        toml::to_string(&self).map_err(anyhow::Error::from)
    }

    #[context("removing the journal")]
    fn remove_from_system(self, host: &HostDirectories) -> anyhow::Result<Vec<InstallOperation>> {
        // Close the file.
        drop(self.file);

        remove_file(&*host.journal_file)?;

        // Update the directory entries so the removal is synced.
        self.directory.sync_all()?;

        // Close the directory.
        drop(self.directory);

        Ok(self.operations)
    }
}

fn create_temporary_files(journal: &Journal, host: &HostDirectories) -> anyhow::Result<()> {
    for operation in &journal.operations {
        let staged = operation.file.with_root(&host.staging);
        let destination = operation.temporary.with_root(&host.installation_root);

        if let Some(parent) = destination.parent() {
            // TODO: Handle directory permissions.
            create_dir_all(parent)?;
        }

        fs::copy(staged, destination)?;
    }

    Ok(())
}

fn create_backups(journal: &Journal, host: &HostDirectories) -> anyhow::Result<()> {
    for operation in &journal.operations {
        let old = operation.file.with_root(&host.installation_root);
        let destination = match &operation.backup {
            Some(path) => path.with_root(&host.installation_root),
            None => continue,
        };

        fs::copy(old, destination)?;
    }

    Ok(())
}

/// Moves the temporary files into their final install locations.
fn switch_to_temporary_files(journal: &Journal, host: &HostDirectories) -> anyhow::Result<()> {
    for operation in &journal.operations {
        let temporary = operation.temporary.with_root(&host.installation_root);
        let destination = operation.file.with_root(&host.installation_root);

        // TODO: Specialise on linux et al. to use rename2e when there is no backup.
        fs::rename(temporary, destination)?;
    }

    Ok(())
}

fn remove_backups(operations: &[InstallOperation], host: &HostDirectories) {
    for operation in operations {
        let backup = match &operation.backup {
            Some(path) => path.with_root(&host.installation_root),
            None => continue,
        };

        remove_file(backup)
            .context("removing backup files")
            .ok_or_log();
    }
}

#[derive(Serialize)]
struct InstallOperation {
    file: Box<TargetPath>,
    temporary: Box<TargetPath>,
    backup: Option<Box<TargetPath>>,
}
