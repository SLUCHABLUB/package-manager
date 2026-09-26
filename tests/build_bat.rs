// TODO: We should set up some better testing fixtures

mod utilities;

use crate::utilities::assert::assert_no_error_logs;
use crate::utilities::assert::assert_no_stdout;
use crate::utilities::assert::assert_success;
use crate::utilities::command::create_command;
use crate::utilities::command::run_command_in;
use crate::utilities::command::set_environment;
use crate::utilities::setup::setup_build;

#[ignore = "TODO"]
#[test]
fn build_bat() {
    let directories = setup_build("build_bat", "bat-manifest.toml");

    let mut command = create_command();

    command.arg("update");

    command.args(["--manifest", "manifest.toml"]);
    command.arg("--root").arg(&directories.system_image);

    // TODO: Don't force landlock here.
    command.args(["--sandbox", "landlock"]);

    set_environment(&mut command, &directories.home);

    let output = run_command_in(command, &directories.test);

    assert_success(&output);
    assert_no_error_logs(&output);
    assert_no_stdout(&output);
}
