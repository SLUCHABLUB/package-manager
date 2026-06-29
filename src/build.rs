use crate::RecipeDirectories;
use crate::fs;
use crate::recipe::BuildSystem;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use std::process::Command;
use tracing::warn;

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

pub fn build(recipe: &Recipe, directories: &RecipeDirectories) -> anyhow::Result<()> {
    fs::make_empty_directory(&directories.build)
        .context("preparing the build's working directory")?;
    fs::make_empty_directory(&directories.target).context("preparing the target directory")?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    warn!("not sand-boxing the build");

    let mut commands = Vec::new();

    match &recipe.build.system {
        // TODO: Should we add the version requirement here?
        // TODO: Should we specify the binary?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo { features, target } => {
            let mut cargo = Command::new("cargo");
            cargo
                .arg("install")
                .arg("--path")
                .arg(&directories.source)
                .arg("--no-track")
                .arg("--root")
                .arg(&directories.target);

            if !features.is_empty() {
                cargo.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                cargo.arg("--target").arg(&**target);
            }

            commands.push(cargo);
        }
        BuildSystem::ConfigureMake { configure_flags } => {
            let cpu_count = num_cpus::get();

            let mut configure = Command::new(directories.source.join("configure"));
            for flag in configure_flags {
                configure.arg(&**flag);
            }

            let mut compile = Command::new("make");
            compile.arg(format!("-j{cpu_count}"));

            let mut install = Command::new("make");
            install
                .arg("install")
                .env(CONFIGURE_MAKE_DISTINATION_DIRECTORY, &directories.target);

            commands.push(configure);
            commands.push(compile);
            commands.push(install);
        }
    };

    for command in &mut commands {
        for (key, value) in &recipe.build.environment_variables {
            command.env(&**key, &**value);
        }
    }

    for command in &mut commands {
        command.current_dir(&directories.build);

        let output = command.output().with_context(|| {
            format!(
                "invoking the `{}` command (full command: `{:?}`)",
                command.get_program().display(),
                command
            )
        })?;

        if !output.status.success() {
            bail!(
                "the `{}` command failed with {}\nfull command:\n{:?}\nstandard output:\n{}\nstandard error:\n{}",
                command.get_program().display(),
                output.status,
                &command,
                output.stdout.as_bstr(),
                output.stderr.as_bstr(),
            )
        }
    }

    Ok(())
}
