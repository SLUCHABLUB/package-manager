mod arguments;
mod build;
mod build_plan;
mod dependencies;
mod directories;
mod download;
mod image;
mod install;
mod ledger;
mod manifest;
mod path;
mod recipe;
mod result;
mod serde;
mod staging;
mod state;
mod version;

pub(crate) use build::ensure_built;
pub(crate) use build_plan::BuildPlan;
pub(crate) use dependencies::check_runtime_dependencies;
pub(crate) use directories::HostDirectories;
pub(crate) use directories::TargetDirectories;
pub(crate) use download::IndexedFile;
pub(crate) use download::detect_tarball_compression;
pub(crate) use download::ensure_downloaded;
pub(crate) use download::find_in_index;
pub(crate) use download::resolve_commit;
pub(crate) use image::make_image;
pub(crate) use install::install;
pub(crate) use ledger::ImageLedger;
pub(crate) use ledger::SystemLedger;
pub(crate) use manifest::Manifest;
pub(crate) use path::HostPath;
pub(crate) use path::TargetPath;
pub(crate) use recipe::BuildSystem;
pub(crate) use recipe::CacheDirectory;
pub(crate) use recipe::Compression;
pub(crate) use recipe::Dependencies;
pub(crate) use recipe::DownloadLock;
pub(crate) use recipe::Recipe;
pub(crate) use recipe::RecipeDirectories;
pub(crate) use result::ResultExtension;
pub(crate) use staging::stage_recipes;
pub(crate) use state::State;
pub(crate) use version::Resolver;
pub(crate) use version::SemanticVersion;
pub(crate) use version::Version;
pub(crate) use version::VersionRequirement;

pub(crate) const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

use anyhow::anyhow;
use arguments::Arguments;
use clap::Parser as _;

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
