use crate::directories::HostDirectories;
use tracing::info;
use tracing::warn;

// TODO: Take an installation method parameter.
#[expect(clippy::unnecessary_wraps, reason = "todo")]
pub(crate) fn install(directories: &HostDirectories) -> anyhow::Result<()> {
    warn!("installing... don't touch the file system please");

    // TODO: Acquire the lock.
    // TODO: Try recover (if the journal exists).

    // TODO: Do a conflict check.
    // TODO: Create the journal (including the ledger).
    // TODO: Create the temporary files.
    // TODO: Create the backups.
    // TODO: Do the rename.
    // TODO: Remove the journal.
    // TODO: Remove the backups.

    let _ = directories;
    warn!("not actually installing :P");

    // TODO: Release the lock.

    info!("done installing; you may touch the file system");

    Ok(())
}
