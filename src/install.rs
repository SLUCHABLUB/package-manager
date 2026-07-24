use crate::HostDirectories;
use crate::PACKAGE_NAME;
use crate::ResultExtension;
use crate::SystemLedger;
use crate::TargetDirectories;
use crate::TargetPath;
use anyhow::Context as _;
use anyhow::bail;
use const_str::concat;
use fn_error_context::context;
use fs_err as fs;
use fs_err::File;
use fs_err::create_dir_all;
use fs_err::remove_file;
use serde::Serialize;
use std::fs::TryLockError;
use std::io::Write;
use tracing::info;
use tracing::warn;

const TEMPORARY_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "temporary");
const BACKUP_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "backup");

// TODO: Take an installation method parameter.
pub(crate) fn install(
    host: &HostDirectories,
    ledger: &SystemLedger,
    target: &TargetDirectories,
) -> anyhow::Result<()> {
    let lock = lock(host)?;

    warn!("installing... don't touch the file system please");

    // TODO: Try recover (if the journal exists).

    let old_ledger = SystemLedger::read_from_host(target)?;

    let mut journal = Journal::new();

    for (recipe, file) in ledger.files() {
        match check_conflict(file, &old_ledger) {
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
                bail!(
                    "the installation of the `{recipe}` recipe would override the unmanaged file at `{file}`"
                );
            }
        }
    }

    journal.operations.push(ledger_install(ledger));

    let journal = journal;

    let serialised_journal = toml::to_string(&journal)?;

    let journal_directory = File::open(&*host.journal_directory)?;
    let mut journal_file = File::create_new(&*host.journal_file)?;

    // TODO: Don't use the try operator beyond this point until we've removed the journal.

    journal_file.write_all(serialised_journal.as_bytes())?;

    journal_file.sync_all()?;
    journal_directory.sync_all()?;

    for operation in &journal.operations {
        let staged = operation.file.with_root(&host.staging);
        let destination = operation.temporary.to_host_path();

        if let Some(parent) = destination.parent() {
            // TODO: Handle directory permissions.
            create_dir_all(parent)?;
        }

        fs::copy(staged, destination)?;
    }

    for operation in &journal.operations {
        let old = operation.file.to_host_path();
        let destination = match &operation.backup {
            Some(path) => path.to_host_path(),
            None => continue,
        };

        fs::copy(old, destination)?;
    }

    for operation in &journal.operations {
        let temporary = operation.temporary.to_host_path();
        let destination = operation.file.to_host_path();

        // TODO: Specialise on linux et al. to use rename2e when there is no backup.
        fs::rename(temporary, destination)?;
    }

    info!("installation complete; cleaning up");

    drop(journal_file);
    remove_file(&*host.journal_file)?;

    journal_directory.sync_all()?;
    drop(journal_directory);

    for operation in &journal.operations {
        let backup = match &operation.backup {
            Some(path) => path.to_host_path(),
            None => continue,
        };

        remove_file(backup).context("removing backups").ok_or_log();
    }

    // If this fails, the kernel will release the lock.
    unlock(lock)?;

    info!("cleaning complete; you may touch the file system");

    Ok(())
}

#[context("acquiring the file system lock")]
fn lock(directories: &HostDirectories) -> anyhow::Result<File> {
    let file = File::create(&*directories.lock_file)?;

    match file.try_lock() {
        Ok(()) => (),
        Err(TryLockError::WouldBlock) => {
            // TODO: Read what we're waiting for.
            warn!("waiting for the file system lock");
            file.lock()?;
        }
        Err(TryLockError::Error(error)) => return Err(error.into()),
    }

    // TODO: Write our PID & boot ID & operation to the file.

    Ok(file)
}

#[context("releasing the file system lock")]
fn unlock(file: File) -> anyhow::Result<()> {
    // TODO: Clear the ID from the file.

    file.unlock()?;

    drop(file);

    Ok(())
}

enum ConflictCheckResult {
    /// The file did not exist in the last generation.
    New,
    /// The file exited in the last generation and has not been modified.
    Updated,
    /// The file existed on the system but was not managed by the package manager.
    Unmanaged,
}

fn check_conflict(file: &TargetPath, old_ledger: &SystemLedger) -> ConflictCheckResult {
    let host_path = file.to_host_path();

    if host_path.exists() {
        if old_ledger.contains(file) {
            // TODO: Check if the hash matches.
            ConflictCheckResult::Updated
        } else {
            ConflictCheckResult::Unmanaged
        }
    } else {
        ConflictCheckResult::New
    }
}

fn ledger_install(ledger: &SystemLedger) -> InstallOperation {
    let should_backup = ledger.path().to_host_path().exists();

    InstallOperation {
        file: Box::from(ledger.path()),
        temporary: ledger.path().with_extension(TEMPORARY_EXTENSION),
        backup: should_backup.then(|| ledger.path().with_extension(BACKUP_EXTENSION)),
    }
}

#[derive(Default, Serialize)]
struct Journal {
    operations: Vec<InstallOperation>,
}

impl Journal {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Serialize)]
struct InstallOperation {
    file: Box<TargetPath>,
    temporary: Box<TargetPath>,
    backup: Option<Box<TargetPath>>,
}
