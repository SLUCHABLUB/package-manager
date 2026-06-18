use anyhow::Context;
use fs_err::read_to_string;
use tracing::error;
use tracing::info;

const MANIFEST_LOCATION: &str = "/etc/package-manager.manifest.toml";

fn main() {
    tracing_subscriber::fmt::init();

    match try_main() {
        Ok(()) => (),
        Err(error) => error!("{:#}", error),
    };

    info!("done");
}

fn try_main() -> anyhow::Result<()> {
    let _: String = read_to_string(MANIFEST_LOCATION).context("reading package manifest")?;

    Ok(())
}
