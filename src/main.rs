mod arguments;
mod result;

use anyhow::anyhow;
use arguments::Arguments;
use clap::Parser;
use package_manager::State;
use package_manager::TargetDirectories;
use result::ResultExtension as _;

fn main() {
    tracing_subscriber::fmt::init();

    let arguments = Arguments::parse();

    try_main(arguments).ok_or_log();
}

#[expect(clippy::needless_pass_by_value)]
fn try_main(arguments: Arguments) -> anyhow::Result<()> {
    // Use a pure rust cryptography provider for rustls to avoid a C-compiler build dependency.
    rustls_rustcrypto::provider()
        .install_default()
        .map_err(|_provider| anyhow!("failed to set the rustls cryptography provider"))?;

    match arguments.action {
        arguments::Action::Install => (),
    }

    let state = State::initialise()?;

    // TODO: Base this on the arguments.
    let target_directories = TargetDirectories::user()?;

    state.install(&target_directories)?;

    Ok(())
}
