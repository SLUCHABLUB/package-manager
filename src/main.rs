mod arguments;
mod build;
mod dependencies;
mod directories;
mod download;
mod image;
mod install;
mod ledger;
mod manifest;
mod path;
mod plan;
mod recipe;
mod recipe_store;
mod result;
mod staging;
mod version;

pub(crate) use build::build;
pub(crate) use dependencies::check_runtime_dependencies;
pub(crate) use directories::HostDirectories;
pub(crate) use directories::TargetDirectories;
pub(crate) use download::IndexedFile;
pub(crate) use download::detect_tarball_compression;
pub(crate) use download::download;
pub(crate) use download::find_in_index;
pub(crate) use download::resolve_commit;
pub(crate) use image::check_image;
pub(crate) use install::install;
pub(crate) use ledger::ImageLedger;
pub(crate) use ledger::SystemLedger;
pub(crate) use manifest::Manifest;
pub(crate) use path::HostPath;
pub(crate) use path::TargetPath;
pub(crate) use recipe::BuildRoot;
pub(crate) use recipe::BuildSystem;
pub(crate) use recipe::BuildWorkingDirectory;
pub(crate) use recipe::Compression;
pub(crate) use recipe::Dependencies;
pub(crate) use recipe::DownloadLock;
pub(crate) use recipe::Image;
pub(crate) use recipe::Recipe;
pub(crate) use recipe::Source;
pub(crate) use recipe::find_cached_download_lock_or_create;
pub(crate) use recipe_store::RecipeStore;
pub(crate) use result::ResultExtension;
pub(crate) use staging::stage;
pub(crate) use version::SemanticVersion;
pub(crate) use version::Version;
pub(crate) use version::VersionRequirement;

pub(crate) const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

use anyhow::anyhow;
use arguments::Action;
use arguments::Arguments;
use clap::Parser as _;

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

    let host_directories = HostDirectories::new()?;
    // TODO: Base this on the manifest.
    let target_directories = TargetDirectories::user()?;

    match arguments.action {
        Action::Update { manifest } => {
            let manifest = HostPath::from_cwd_relative(&manifest)?;
            let manifest = Manifest::read_from(manifest)?;

            let _old_ledger = SystemLedger::read_from_host(&target_directories)?;
            let recipes = leak(manifest.create_recipe_store());

            let download_plan = manifest.update(recipes, &host_directories)?;
            let build_plan = download_plan.download(&host_directories)?;
            let check_plan = build_plan.build(&target_directories, &host_directories)?;
            let stage_plan = check_plan.check()?;
            let new_ledger = stage_plan.stage(&host_directories, &target_directories)?;

            install(new_ledger, &host_directories, &target_directories)?;
        }
    }

    Ok(())
}

// Since this is just a (non long-lived) executable,
// leaking some things in main is fine and just makes some types nicer.
fn leak<T: 'static>(thing: T) -> &'static T {
    Box::leak(Box::new(thing))
}
