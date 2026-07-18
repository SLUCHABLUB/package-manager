use crate::BuildSystem;
use crate::Recipe;
use crate::State;
use crate::recipe::Build;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use fs_err as fs;
use std::env::home_dir;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tracing::warn;

// TODO: Create HostPath and TargetPath wrapper structs.

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

struct BuildInstruction<'data> {
    commands: Vec<Command>,
    working_directory: &'data Path,

    copies: Vec<FileTransfer<PathBuf, PathBuf>>,
}

struct FileTransfer<FromPath, ToPath> {
    from: FromPath,
    to: ToPath,
}

#[derive(Copy, Clone)]
enum Sandbox {
    None,
}

#[context("building the `{}` recipe", recipe.name)]
pub(crate) fn build(recipe: &Recipe, target_directory: &Path, state: &State) -> anyhow::Result<()> {
    let build_root = recipe.directories.build_root(recipe, state)?;
    let working_directory = recipe.directories.build_working(recipe, state)?;

    for (dependency, version) in &recipe.build.dependencies.versions {
        // TODO
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    // TODO: Generate this from `recipe.install`.
    let mut copies = Vec::new();

    let commands = generate_commands(
        &recipe.build,
        build_root,
        working_directory,
        target_directory,
        &mut copies,
    );

    let instruction = BuildInstruction {
        commands,
        working_directory,
        copies: Vec::new(),
    };

    // TODO: Base the sandbox on the manifest.
    build_in_sandbox(instruction, Sandbox::None)?;

    Ok(())
}

// TODO: Fix this parameter hazard.
fn generate_commands(
    build: &Build,
    build_root: &Path,
    working_directory: &Path,
    target_directory: &Path,
    copies: &mut Vec<FileTransfer<PathBuf, PathBuf>>,
) -> Vec<Command> {
    let mut commands = Vec::new();

    // TODO: Don't hardcode this.
    // TODO: Get this PESKY unwrap the heck outta here.
    let executables_prefix = home_dir().unwrap().join(".local/bin");

    // TODO: Pass the right prefixes.
    match &build.system {
        BuildSystem::None => (),
        // TODO: Should we add the version requirement here?
        // TODO: Should we specify the binary?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo {
            binary,
            features,
            target,
        } => {
            let cargo_target_directory = working_directory.join("target");

            let mut cargo = Command::new("cargo");
            cargo
                .arg("build")
                .arg("--bin")
                .arg(&**binary)
                .arg("--locked")
                .arg("--release")
                .arg("--path")
                .arg(build_root)
                .arg("--target")
                .arg(&cargo_target_directory);

            if !features.is_empty() {
                cargo.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                cargo.arg("--target").arg(&**target);
            }

            // TODO: Use `--artefact-dir` when it gets stabilised.
            let artefact_path = cargo_target_directory.join("release").join(&**binary);
            let artefact_target_path = executables_prefix.join(&**binary);

            copies.push(FileTransfer {
                from: artefact_path,
                to: artefact_target_path,
            });

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
fn build_in_sandbox(mut instruction: BuildInstruction, sandbox: Sandbox) -> anyhow::Result<()> {
    match sandbox {
        Sandbox::None => (),
    }

    warn!("not sand-boxing the build");

    for command in &mut instruction.commands {
        command.current_dir(instruction.working_directory);

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

    for copy in instruction.copies {
        fs::copy(copy.from, copy.to)?;
    }

    Ok(())
}
