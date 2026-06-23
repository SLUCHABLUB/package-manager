use crate::Directories;
use crate::recipe::BuildSystem;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use std::path::Path;
use std::process::Command;
use tracing::warn;

pub fn build(recipe: &Recipe, source: &Path, target: &Path) -> anyhow::Result<()> {
    Directories::make_empty(target).context("preparing the target directory")?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        warn!("TODO: install `{dependency}` version {version} as a build dependency");
    }

    warn!("TODO: Sandbox the build");

    let mut command;

    match &recipe.build.system {
        // TODO: Should we add the version requirement here?
        // TODO: Should we specify the binary?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo { features } => {
            command = Command::new("cargo");
            command
                .arg("install")
                .arg("--path")
                .arg(source)
                .arg("--no-track")
                .arg("--root")
                .arg(target)
                .arg("--features")
                .arg(features.join(" "));
        }
    };

    build_with(&mut command).with_context(|| format!("building the package with: {command:?}"))
}

pub fn build_with(command: &mut Command) -> anyhow::Result<()> {
    let output = command
        .output()
        .context("failed to invoke the build command")?;

    if !output.status.success() {
        bail!(
            "the build command failed with exit code {}\nstandard output:\n{}\nstandard error:\n{}",
            output.status,
            output.stdout.as_bstr(),
            output.stderr.as_bstr(),
        )
    }

    Ok(())
}
