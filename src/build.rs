use crate::RecipeDirectories;
use crate::recipe::BuildSystem;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use std::process::Command;
use tracing::info;
use tracing::warn;

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

#[context("building the `{}` recipe", recipe.name)]
pub(crate) fn build(recipe: &Recipe, directories: &RecipeDirectories) -> anyhow::Result<()> {
    let Some(target_directory) = directories.target()?.as_unpopulated() else {
        info!("using cached target");
        return Ok(());
    };

    let build_root = directories.build_root()?;
    let working_directory = directories.build_working()?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    warn!("not sand-boxing the build");

    let mut commands = Vec::new();

    match &recipe.build.system {
        BuildSystem::None => (),
        // TODO: Should we add the version requirement here?
        // TODO: Should we specify the binary?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo { features, target } => {
            let mut cargo = Command::new("cargo");
            cargo
                .arg("install")
                .arg("--path")
                .arg(build_root)
                .arg("--no-track")
                .arg("--root")
                .arg(target_directory);

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

            let mut configure = Command::new(build_root.join("configure"));
            for flag in configure_flags {
                configure.arg(&**flag);
            }

            let mut compile = Command::new("make");
            compile.arg(format!("-j{cpu_count}"));

            let mut install = Command::new("make");
            install
                .arg("install")
                .env(CONFIGURE_MAKE_DISTINATION_DIRECTORY, target_directory);

            commands.push(configure);
            commands.push(compile);
            commands.push(install);
        }
    }

    for command in &mut commands {
        for (key, value) in &recipe.build.environment_variables {
            command.env(&**key, &**value);
        }
    }

    for command in &mut commands {
        command.current_dir(working_directory);

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
