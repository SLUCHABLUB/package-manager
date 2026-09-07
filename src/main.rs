mod arguments;
mod build;
mod dependencies;
mod directories;
mod download;
mod hash;
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
pub(crate) use hash::hash;
pub(crate) use image::check_image;
pub(crate) use install::install;
pub(crate) use ledger::ImageLedger;
pub(crate) use ledger::SystemLedger;
pub(crate) use manifest::Manifest;
pub(crate) use path::HostPath;
pub(crate) use path::TargetPath;
pub(crate) use plan::LockPlan;
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

use crate::build::Sandbox;
use crate::result::log_error;
use anyhow::anyhow;
use arguments::Action;
use arguments::Arguments;
use clap::Parser as _;
use std::io::stderr;
use std::process::ExitCode;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub(crate) const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log_error(error);
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> anyhow::Result<()> {
    let arguments = Arguments::try_parse()?;

    set_up_tracing()?;

    install_cryptography()?;

    match arguments.action {
        Action::Update {
            manifest: relative_manifest_path,
            root,
            sandbox,
        } => {
            let absolute_manifest_path = HostPath::from_cwd_relative(&relative_manifest_path)?;
            let root = HostPath::from_cwd_relative(&root)?;

            update(absolute_manifest_path, root, sandbox)?;
        }
    }

    Ok(())
}

// Since this is just a (non long-lived) executable,
// leaking some things in main is fine and just makes some types nicer.
fn leak<T: 'static>(thing: T) -> &'static T {
    Box::leak(Box::new(thing))
}

fn set_up_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(stderr)
        .try_init()
        .map_err(anyhow::Error::from_boxed)?;

    Ok(())
}

fn install_cryptography() -> anyhow::Result<()> {
    // Use a pure rust cryptography provider for rustls to avoid a C-compiler build dependency.
    rustls_rustcrypto::provider()
        .install_default()
        .map_err(|_provider| anyhow!("failed to set the rustls cryptography provider"))
}

fn update(
    manifest_path: Box<HostPath>,
    installation_root: Box<HostPath>,
    sandbox: Sandbox,
) -> anyhow::Result<()> {
    let manifest = Manifest::read_from(manifest_path)?;

    let target_directories = manifest.target_directories()?;

    let host_directories = HostDirectories::new(&target_directories, installation_root)?;

    let recipes = leak(manifest.create_recipe_store()?);

    let lock_plan = manifest.lock_plan(recipes)?;
    let hash_plan = lock_plan.lock(&host_directories)?;
    let download_plan = hash_plan.hash();
    let build_plan = download_plan.download(&host_directories)?;
    let check_plan = build_plan.build(sandbox, &target_directories, &host_directories)?;
    let stage_plan = check_plan.check()?;
    let new_ledger = stage_plan.stage(&host_directories, &target_directories)?;

    install(new_ledger, &host_directories, &target_directories)?;

    Ok(())
}
