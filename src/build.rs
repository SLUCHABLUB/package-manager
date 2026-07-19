use crate::BuildSystem;
use crate::Directories;
use crate::Recipe;
use crate::State;
use crate::recipe::Build;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use fs_err as fs;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tracing::warn;

// TODO: Create HostPath and TargetPath wrapper structs.

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

#[derive(Debug)]
struct BuildInstruction<'data> {
    commands: Vec<Command>,
    working_directory: &'data Path,

    copies: Vec<FileTransfer<PathBuf, PathBuf>>,
}

#[derive(Debug)]
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
    )?;

    let instruction = BuildInstruction {
        commands,
        working_directory,
        copies,
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
) -> anyhow::Result<Vec<Command>> {
    let mut commands = Vec::new();

    let target_directories = Directories::user()?;

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
            let cargo_manifest_path = build_root.join("Cargo.toml");
            let cargo_target_directory = working_directory.join("target");

            let mut cargo = Command::new("cargo");
            cargo
                .arg("build")
                .arg("--bin")
                .arg(&**binary)
                .arg("--locked")
                .arg("--release")
                .arg("--manifest-path")
                .arg(cargo_manifest_path)
                .arg("--target-dir")
                .arg(&cargo_target_directory);

            if !features.is_empty() {
                cargo.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                cargo.arg("--target").arg(&**target);
            }

            let artefact_path = cargo_target_directory.join("release").join(&**binary);
            // TODO: Unshitify this expression when we have proper types.
            let artefact_target_path = target_directory.join(
                target_directories
                    .executables
                    .join(&**binary)
                    .strip_prefix("/")
                    .context("stripping `/`-prefix from an absolute path")?,
            );

            copies.push(FileTransfer {
                from: artefact_path,
                to: artefact_target_path,
            });

            commands.push(cargo);
        }
        BuildSystem::ConfigureMake { configure_flags } => {
            let cpu_count = num_cpus::get();

            let mut configure = Command::new(build_root.join("configure"));

            configure.arg(concat_os("--prefix=", &target_directories.prefix));
            configure.arg(concat_os("--bindir=", &target_directories.executables));
            // TODO: Maybe set "sbindir"?
            configure.arg(concat_os(
                "--libexecdir=",
                &target_directories.internal_executables,
            ));
            configure.arg(concat_os("--datarootdir=", &target_directories.data));
            configure.arg(concat_os("--datadir=", &target_directories.data));
            configure.arg(concat_os(
                "--sysconfdir=",
                &target_directories.configuration,
            ));
            configure.arg(concat_os("--sharedstatedir=", &target_directories.state));
            configure.arg(concat_os("--localstatedir=", &target_directories.state));
            configure.arg(concat_os("--runstatedir=", &target_directories.runtime));
            configure.arg(concat_os("--includedir=", &target_directories.headers));
            configure.arg(concat_os("--libdir=", &target_directories.libraries));

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

    Ok(commands)
}

fn concat_os(prefix: &str, suffix: impl AsRef<OsStr>) -> OsString {
    let mut string = OsString::from(prefix);
    string.push(suffix);
    string
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
        if let Some(parent) = copy.to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(copy.from, copy.to)?;
    }

    Ok(())
}
