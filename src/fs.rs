use fs_err::create_dir_all;
use fs_err::remove_dir_all;
use std::io;
use std::path::Path;

pub fn make_empty_directory(directory: &Path) -> anyhow::Result<()> {
    match remove_dir_all(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }?;
    create_dir_all(directory)?;

    Ok(())
}
