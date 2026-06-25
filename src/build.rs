use crate::RecipeDirectories;
use crate::fs;
use crate::recipe::BuildSystem;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use std::process::Command;
use tracing::warn;

pub fn build(recipe: &Recipe, directories: &RecipeDirectories) -> anyhow::Result<()> {
    fs::make_empty_directory(&directories.target).context("preparing the target directory")?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    warn!("not sand-boxing the build");

    let mut command;

    match &recipe.build.system {
        // TODO: Should we add the version requirement here?
        // TODO: Should we specify the binary?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo {
            features,
            target,
            environment_variables,
        } => {
            command = Command::new("cargo");
            command
                .arg("install")
                .arg("--path")
                .arg(&directories.source)
                .arg("--no-track")
                .arg("--root")
                .arg(&directories.target);

            for (key, value) in environment_variables {
                command.env(&**key, &**value);
            }

            if !features.is_empty() {
                command.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                command.arg("--target").arg(&**target);
            }
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
            "the build command failed with {}\nstandard output:\n{}\nstandard error:\n{}",
            output.status,
            output.stdout.as_bstr(),
            output.stderr.as_bstr(),
        )
    }

    Ok(())
}
