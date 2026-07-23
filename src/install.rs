use crate::SystemLedger;
use crate::TargetPath;
use crate::directories::HostDirectories;
use anyhow::bail;
use fn_error_context::context;
use fs_err::File;
use std::fs::TryLockError;
use tracing::info;
use tracing::warn;

// TODO: Take an installation method parameter.
pub(crate) fn install(directories: &HostDirectories, ledger: SystemLedger) -> anyhow::Result<()> {
    let lock = lock(directories)?;

    warn!("installing... don't touch the file system please");

    // TODO: Try recover (if the journal exists).

    for (recipe, ledger) in ledger.recipes {
        for file in ledger.files {
            // TODO: Record this in the journal.
            match check_conflict(&file) {
                ConflictCheckResult::New => (),
                ConflictCheckResult::Unmanaged => {
                    // TODO: We could prompt the user here.
                    bail!(
                        "the installation of the `{recipe}` recipe would override the unmanaged file at `{file}`"
                    );
                }
            }
        }
    }

    // TODO: Create the journal (including the ledger).
    // TODO: Create the temporary files.
    // TODO: Create the backups.
    // TODO: Do the rename.
    // TODO: Remove the journal.
    // TODO: Remove the backups.

    let _ = directories;
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
