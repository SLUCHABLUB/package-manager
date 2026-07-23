use crate::SystemLedger;
use crate::directories::HostDirectories;
use fn_error_context::context;
use fs_err::File;
use std::fs::TryLockError;
use tracing::info;
use tracing::warn;

// TODO: Take an installation method parameter.
pub(crate) fn install(directories: &HostDirectories, ledger: SystemLedger) -> anyhow::Result<()> {
    warn!("installing... don't touch the file system please");

    let lock = lock(directories)?;

    // TODO: Try recover (if the journal exists).

    drop(ledger);

    // TODO: Do a conflict check.
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
