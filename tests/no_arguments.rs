mod assert;

use crate::assert::assert_failure;
use crate::assert::assert_no_bad_logs;
use crate::assert::assert_no_stdout;
use assert::ResultExtension as _;
use assert_cmd::Command;

#[test]
fn no_arguments() {
    let output = Command::cargo_bin("package-manager")
        .assert_ok()
        .output()
        .assert_ok();

    assert_failure(&output);
    assert_no_bad_logs(&output);
    assert_no_stdout(&output);
}
