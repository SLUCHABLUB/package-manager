use crate::PACKAGE_NAME;
use crate::ResultExtension;
use crate::SystemLedger;
use crate::TargetPath;
use crate::directories::HostDirectories;
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

const TYEMPORARY_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "temporary");
//const BACKUP_EXTENSION: &str = concat!(PACKAGE_NAME, '-', "backup");

// TODO: Take an installation method parameter.
pub(crate) fn install(directories: &HostDirectories, ledger: SystemLedger) -> anyhow::Result<()> {
    let lock = lock(directories)?;

    warn!("installing... don't touch the file system please");

    // TODO: Try recover (if the journal exists).

    let mut journal = Journal::new();

    for (recipe, ledger) in ledger.recipes {
        for file in ledger.files {
            // TODO: Record this in the journal.
            match check_conflict(&file) {
                ConflictCheckResult::New => {
                    let temporary = file.with_extension(TYEMPORARY_EXTENSION);

                    journal.operations.push(InstallOperation {
                        file,
                        temporary,
                        backup: None,
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
    }

    let journal = journal;

    let serialised_journal = toml::to_string(&journal)?;

    let journal_directory = File::open(&*directories.journal_directory)?;
    let mut journal_file = File::create_new(&*directories.journal_file)?;

    // TODO: Don't use the try operator beyond this point until we've removed the journal.

    journal_file.write_all(serialised_journal.as_bytes())?;

    journal_file.sync_all()?;
    journal_directory.sync_all()?;

    for operation in &journal.operations {
        let staged = operation.file.with_root(&directories.staging);
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

    drop(journal_file);
    remove_file(&*directories.journal_file)?;

    journal_directory.sync_all()?;
    drop(journal_directory);

    for operation in &journal.operations {
        let backup = match &operation.backup {
            Some(path) => path.to_host_path(),
            None => continue,
        };

        remove_file(backup).context("removing backups").ok_or_log();
    }

    warn!("not actually installing :P");

    // If this fails, the kernel will release the lock.
    unlock(lock)?;

    info!("done installing; you may touch the file system");

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
    /// The file existed on the system but was not managed by the package manager.
    Unmanaged,
}

fn check_conflict(file: &TargetPath) -> ConflictCheckResult {
    let file = file.to_host_path();

    if file.exists() {
        // TODO: Check if the file matches the old ledger.

        return ConflictCheckResult::Unmanaged;
    }

    ConflictCheckResult::New
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
