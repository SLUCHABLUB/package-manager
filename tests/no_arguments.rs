use assert_cmd::Command;

#[test]
fn no_arguments() {
    Command::cargo_bin("package-manager")
        .unwrap()
        .assert()
        .code(2);
}
