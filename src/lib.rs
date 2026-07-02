pub mod recipe;

mod build;
mod build_plan;
mod dependencies;
mod directories;
mod download;
mod ledger;
mod manifest;
mod prepare_to_install;
mod result;
mod state;
mod version;

pub(crate) use build::build;
pub(crate) use build_plan::BuildPlan;
pub(crate) use directories::RecipeDirectories;
pub(crate) use download::download;
pub(crate) use ledger::Ledger;
pub(crate) use manifest::Manifest;
pub(crate) use prepare_to_install::prepare_to_install;
pub(crate) use recipe::Recipe;
pub(crate) use result::ResultExtension;
pub(crate) use version::Version;
pub(crate) use version::VersionRequirement;

pub use state::State;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
