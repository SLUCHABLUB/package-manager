#![allow(
    dead_code,
    reason = "cargo doesn't see that other modules use this one"
)]

use crate::utilities::assert::ResultExtension as _;
use fs_err::create_dir_all;
use std::io;
use std::os::unix;
use std::path::Path;
use std::path::PathBuf;

fn symlink(original: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    // Symlinks are freaky on some weird OSes.
    match unix::fs::symlink(original, to) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        result => result,
    }
}

pub struct BuildDirectories {
    pub test: PathBuf,
    pub system_image: PathBuf,
    pub home: PathBuf,
}

pub fn setup_build(directory: &str, manifest_name: &str) -> BuildDirectories {
    let test_directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(directory);
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets");

    let system_image = test_directory.join("system_image");
    let home = test_directory.join("home");

    create_dir_all(&system_image).assert_ok();
    create_dir_all(&home).assert_ok();

    symlink(
        assets.join(manifest_name),
        test_directory.join("manifest.toml"),
    )
    .assert_ok();
    if !test_directory.join("recipes").exists() {
        symlink(assets.join("recipes"), test_directory.join("recipes")).assert_ok();
    }

    BuildDirectories {
        test: test_directory,
        system_image,
        home,
    }
}
