#![allow(
    dead_code,
    reason = "cargo doesn't see that other modules use this one"
)]

use crate::utilities::assert::ResultExtension;
use std::io;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Output;
use std::thread;
use std::thread::JoinHandle;

pub(crate) fn create_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_package-manager"))
}

pub(crate) fn run_command(mut command: Command) -> Output {
    let mut child = command.spawn().assert_ok();

    let child_stderr = child.stderr.take().expect("stderr should exist");
    let child_stdout = child.stdout.take().expect("stdout should exist");

    let stdout_capture = capture_output(child_stdout, io::stdout());
    let stderr_capture = capture_output(child_stderr, io::stderr());

    let status = child.wait().assert_ok();

    Output {
        status,
        stdout: stdout_capture.join().unwrap(),
        stderr: stderr_capture.join().unwrap(),
    }
}

fn capture_output(
    mut output: impl Read + Send + 'static,
    mut tty: impl Write + Send + 'static,
) -> JoinHandle<Vec<u8>> {
    /// A completely arbitrary number.
    const BUFFER_SIZE: usize = 1024;

    thread::spawn(move || {
        let mut buffer = [0; BUFFER_SIZE];
        let mut capture = Vec::new();

        loop {
            let data_length = output.read(&mut buffer).assert_ok();

            if data_length == 0 {
                break;
            }

            let read_data = &buffer[..data_length];

            tty.write_all(read_data).assert_ok();
            capture.write_all(read_data).assert_ok();
        }

        capture
    })
}
