mod utilities;

use crate::utilities::assert::ResultExtension as _;
use crate::utilities::assert::assert_failure;
use crate::utilities::assert::assert_no_bad_logs;
use crate::utilities::assert::assert_no_stdout;
use crate::utilities::command::create_command;

#[test]
fn no_arguments() {
    let output = create_command().output().assert_ok();

    assert_failure(&output);
    assert_no_bad_logs(&output);
    assert_no_stdout(&output);
}
