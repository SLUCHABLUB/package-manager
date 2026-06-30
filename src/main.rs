mod arguments;
mod result;

use crate::arguments::Arguments;
use anyhow::anyhow;
use clap::Parser;
use package_manager::State;
use result::ResultExtension as _;

fn main() {
    tracing_subscriber::fmt::init();

    let arguments = Arguments::parse();

    try_main(arguments).ok_or_log();
}

fn try_main(arguments: Arguments) -> anyhow::Result<()> {
    // Use a pure rust cryptography provider for rustls to avoid a C-compiler build dependency.
    rustls_rustcrypto::provider()
        .install_default()
        .map_err(|_provider| anyhow!("failed to set the rustls cryptography provider"))?;

    let state = State::initialise(&arguments.manifest)?;

    state.prepare_to_install()?;

    Ok(())
}
