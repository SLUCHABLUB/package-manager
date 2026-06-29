pub mod recipe;

mod build;
mod dependencies;
mod directories;
mod download;
mod fs;
mod ledger;
mod manifest;
mod prepare_install;
mod recipe_set;
mod result;
mod version;

pub use build::build;
pub use directories::RecipeDirectories;
pub use download::download;
pub use ledger::Ledger;
pub use manifest::Manifest;
pub use prepare_install::prepare_install;
pub use recipe_set::RecipeSet;
pub use result::ResultExtension;
pub use version::Version;
pub use version::VersionRequirement;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
