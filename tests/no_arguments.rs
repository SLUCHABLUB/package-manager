mod utilities;

use crate::utilities::assert::assert_failure;
use crate::utilities::assert::assert_no_bad_logs;
use crate::utilities::assert::assert_no_stdout;
use crate::utilities::command::create_command;
use crate::utilities::command::run_command;

#[test]
fn no_arguments() {
    let output = run_command(create_command());

    assert_failure(&output);
    assert_no_bad_logs(&output);
    assert_no_stdout(&output);
}
