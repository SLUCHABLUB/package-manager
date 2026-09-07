// TODO: We should set up some better testing fixtures

mod utilities;

use crate::utilities::assert::ResultExtension as _;
use crate::utilities::assert::assert_no_error_logs;
use crate::utilities::assert::assert_no_stdout;
use crate::utilities::assert::assert_success;
use crate::utilities::command::create_command;
use crate::utilities::command::run_command;
use fs_err::create_dir_all;
use std::env;
use std::io;
use std::os::unix;
use std::path::Path;
use std::process::Command;

fn symlink(original: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    // Symlinks are freaky on some weird OSes.
    match unix::fs::symlink(original, to) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        result => result,
    }
}

#[test]
fn build_bat() {
    let test_directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("build_bat");
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets");

    let root = test_directory.join("system_image");
    let home = test_directory.join("home");

    create_dir_all(&root).assert_ok();
    create_dir_all(&home).assert_ok();

    symlink(
        assets.join("bat-manifest.toml"),
        test_directory.join("manifest.toml"),
    )
    .assert_ok();
    if !test_directory.join("recipes").exists() {
        symlink(assets.join("recipes"), test_directory.join("recipes")).assert_ok();
    }

    let mut command = create_command();

    command.arg("update");

    command.args(["--manifest", "manifest.toml"]);
    command.arg("--root").arg(&root);

    // TODO: Don't force landlock here.
    command.args(["--sandbox", "landlock"]);

    command.current_dir(&test_directory);

    set_environment(&mut command, &test_directory);

    let output = run_command(command);

    assert_success(&output);
    assert_no_error_logs(&output);
    assert_no_stdout(&output);
}

fn set_environment(command: &mut Command, test_directory: &Path) {
    command.env_clear();

    command.env("HOME", test_directory.join("home"));

    // TODO: Figure out what to do with this.
    propagate_env_var("PATH", command);

    propagate_env_var("RUST_BACKTRACE", command);
    propagate_env_var("RUST_LIB_BACKTRACE", command);
    propagate_env_var("RUST_LOG", command);
}

fn propagate_env_var(variable: &str, command: &mut Command) {
    if let Some(value) = env::var_os(variable) {
        command.env(variable, value);
    }
}
