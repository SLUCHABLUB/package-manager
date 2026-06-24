pub mod recipe;

mod build;
mod dependencies;
mod directories;
mod download;
mod fs;
mod ledger;
mod prepare_install;
mod version;

pub use build::build;
pub use directories::RecipeDirectories;
pub use download::download;
pub use ledger::Ledger;
pub use prepare_install::prepare_install;
pub use version::Version;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
