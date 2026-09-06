// TODO: We should set up some better testing fixtures

mod assert;

use assert::ResultExtension as _;
use assert_cmd::cargo::CommandCargoExt as _;
use bstr::ByteSlice;
use fs_err::create_dir_all;
use std::env;
use std::io;
use std::os::unix;
use std::path::Path;
use std::process::Command;
use std::process::Output;
use tap::Pipe as _;

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

    let mut command = Command::cargo_bin("package-manager").unwrap();

    command.arg("update");

    command.args(["--manifest", "manifest.toml"]);
    command.arg("--root").arg(&root);

    command.current_dir(&test_directory);

    set_environment(&mut command, &test_directory);

    command.output().assert_ok().pipe(check_output);
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

fn check_output(output: Output) {
    let Output {
        status,
        stdout,
        stderr,
    } = output;

    assert!(status.success());

    assert!(stdout.is_empty(), "{}", String::from_utf8_lossy(&stdout));

    // TODO: Disallow warnings too.
    assert!(
        !stderr.contains_str("ERROR"),
        "{}",
        String::from_utf8_lossy(&stderr)
    );
}
