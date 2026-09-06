#![allow(
    dead_code,
    reason = "cargo doesn't see that other modules use this one"
)]

use bstr::ByteSlice;
use std::process::Output;

pub(crate) trait ResultExtension {
    type T;
    type E;

    #[track_caller]
    fn assert_ok(self) -> Self::T;
}

impl<T, E> ResultExtension for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    type T = T;
    type E = E;

    #[track_caller]
    fn assert_ok(self) -> T {
        match self {
            Ok(ok) => ok,
            Err(error) => panic!("{:#}", error.into()),
        }
    }
}

#[track_caller]
pub(crate) fn assert_no_stdout(output: &Output) {
    assert!(output.stdout.is_empty());
}
#[track_caller]
pub(crate) fn assert_success(output: &Output) {
    assert!(output.status.success());
}

#[track_caller]
pub(crate) fn assert_failure(output: &Output) {
    assert!(!output.status.success());
}

pub(crate) fn assert_no_bad_logs(output: &Output) {
    assert_no_error_logs(output);
    assert_no_warning_logs(output);
}

// TODO: Make this private when warnings are avoidable.
#[track_caller]
pub(crate) fn assert_no_error_logs(output: &Output) {
    assert!(!output.stderr.contains_str("ERROR"));
}

#[track_caller]
fn assert_no_warning_logs(output: &Output) {
    assert!(!output.stderr.contains_str("WARN"));
}
