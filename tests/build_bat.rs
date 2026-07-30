// TODO: We should set up some better testing fixtures

mod assert;

use assert::ResultExtension as _;
use assert_cmd::cargo::CommandCargoExt as _;
use fs_err as fs;
use fs_err::create_dir_all;
use std::env;
use std::path::Path;
use std::process::Command;
use tap::Pipe as _;

#[test]
fn build_bat() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("build-bat-root");
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets");

    create_dir_all(&root).assert_ok();

    fs::copy(assets.join("bat-manifest.toml"), root.join("manifest.toml")).assert_ok();
    if !root.join("recipes").exists() {
        std::os::unix::fs::symlink(assets.join("recipes"), root.join("recipes")).assert_ok();
    }

    Command::cargo_bin("package-manager")
        .unwrap()
        .args(["update", "--manifest", "manifest.toml"])
        .env_clear()
        .current_dir(&root)
        .envs([
            ("HOME", root.join("home")),
            ("XDG_BIN_HOME", root.join("executables")),
            ("XDG_CACHE_HOME", root.join("cache")),
            ("XDG_CONFIG_HOME", root.join("configuration")),
            ("XDG_DATA_HOME", root.join("data")),
            ("XDG_INCLUDE_HOME", root.join("headers")),
            ("XDG_LIB_HOME", root.join("libraries")),
            ("XDG_STATE_HOME", root.join("state")),
            ("XDG_RUNTIME_DIR", root.join("run")),
        ])
        // TODO: Figure out what to do with this.
        .env("PATH", env!("PATH"))
        .envs(
            [
                env::var_os("RUST_BACKTRACE").map(|value| ("RUST_BACKTRACE", value)),
                env::var_os("RUST_LIB_BACKTRACE").map(|value| ("RUST_LIB_BACKTRACE", value)),
            ]
            .into_iter()
            .flatten(),
        )
        .status()
        .assert_ok()
        .success()
        .pipe(|success| assert!(success));
}
