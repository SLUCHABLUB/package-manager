use crate::HostDirectories;
use crate::ResultExtension;
use fn_error_context::context;
use fs_err::File;
use serde::Deserialize;
use serde::Serialize;
use show_option::ShowOption as _;
use std::fs::TryLockError;
use std::io::Seek as _;
use std::io::SeekFrom;
use std::io::Write as _;
use std::io::read_to_string;
use std::process;
use tracing::warn;

#[context("acquiring the file system lock")]
pub(super) fn lock(directories: &HostDirectories) -> anyhow::Result<File> {
    let file_path = &*directories.lock_file;

    let lock = LockFile::for_this_process();
    let serialised_lock = lock.serialise()?;

    let mut file = File::options()
        .create(true)
        .read(true)
        .write(true)
        .open(file_path)?;

    match file.try_lock() {
        Ok(()) => (),
        Err(TryLockError::WouldBlock) => {
            let their_pid = LockFile::read_from_file(&file)
                .map(|lock| lock.process_id)
                .ok_or_log();

            warn!(
                "waiting for the file system lock (held by {})",
                their_pid.show_prefixed_or("the process with id ", "some unknown process")
            );
            file.lock()?;
        }
        Err(TryLockError::Error(error)) => return Err(error.into()),
    }

    clear_file(&mut file)?;

    file.write_all(serialised_lock.as_ref())?;

    Ok(file)
}

fn clear_file(file: &mut File) -> anyhow::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;

    Ok(())
}

#[context("releasing the file system lock")]
pub(super) fn unlock(file: File) -> anyhow::Result<()> {
    file.set_len(0)?;

    file.unlock()?;

    drop(file);

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct LockFile {
    process_id: u32,
}

impl LockFile {
    fn for_this_process() -> LockFile {
        LockFile {
            process_id: process::id(),
        }
    }

    fn serialise(&self) -> anyhow::Result<impl AsRef<[u8]> + use<>> {
        toml::to_string(&self).map_err(anyhow::Error::from)
    }

    fn read_from_file(file: &File) -> anyhow::Result<LockFile> {
        let serialised = read_to_string(file)?;
        let lock = toml::from_str(&serialised)?;

        Ok(lock)
    }
}
