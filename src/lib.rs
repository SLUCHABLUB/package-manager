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

pub(crate) use build_plan::BuildPlan;
pub(crate) use result::ResultExtension;

pub use build::build;
pub use directories::RecipeDirectories;
pub use download::download;
pub use ledger::Ledger;
pub use manifest::Manifest;
pub use prepare_to_install::prepare_to_install;
pub use recipe::Recipe;
pub use state::State;
pub use version::Version;
pub use version::VersionRequirement;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
