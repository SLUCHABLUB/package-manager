use crate::BuildSystem;
use crate::Recipe;
use crate::State;
use crate::recipe::Build;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use std::path::Path;
use std::process::Command;
use tracing::warn;

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

#[context("building the `{}` recipe", recipe.name)]
pub(crate) fn build(recipe: &Recipe, target_directory: &Path, state: &State) -> anyhow::Result<()> {
    let build_root = recipe.directories.build_root(recipe, state)?;
    let working_directory = recipe.directories.build_working(recipe, state)?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        // TODO
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    let mut commands = generate_commands(&recipe.build, build_root, target_directory);

    run_commands(&mut commands, working_directory)?;

    Ok(())
}

fn generate_commands(build: &Build, build_root: &Path, target_directory: &Path) -> Vec<Command> {
    let mut commands = Vec::new();

    // TODO: Pass the right prefixes.
    match &build.system {
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
        for (key, value) in &build.environment_variables {
            command.env(&**key, &**value);
        }
    }

    commands
}

// TODO: Add a sandbox parameter.
fn run_commands(commands: &mut [Command], working_directory: &Path) -> anyhow::Result<()> {
    warn!("not sand-boxing the build");

    for command in commands {
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
                command,
                output.stdout.as_bstr(),
                output.stderr.as_bstr(),
            )
        }
    }

    Ok(())
}
