use crate::BuildRoot;
use crate::BuildSystem;
use crate::BuildWorkingDirectory;
use crate::HostDirectories;
use crate::HostPath;
use crate::Image;
use crate::Recipe;
use crate::Source;
use crate::TargetDirectories;
use crate::TargetPath;
use crate::recipe::Build;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use fs_err as fs;
use fs_err::create_dir_all;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use tracing::info;
use tracing::warn;

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

#[derive(Debug)]
struct FileTransfer {
    from: Box<HostPath>,
    to: Box<TargetPath>,
}

#[derive(Copy, Clone)]
enum Sandbox {
    None,
}

#[context("building {recipe}")]
pub(crate) fn build(
    recipe: &Recipe,
    source: Source,
    target_directories: &TargetDirectories,
    host: &HostDirectories,
) -> anyhow::Result<Image> {
    let image = match Image::find_cached(recipe, host)? {
        Ok(cached) => return Ok(cached),
        Err(unitialised) => unitialised,
    };

    info!("building {}", recipe.name());

    create_dir_all(&image)?;

    let build_root = BuildRoot::new(source, recipe)?;
    let working_directory = BuildWorkingDirectory::new(recipe, host)?;

    for (dependency, version) in &recipe.build_data().dependencies.versions {
        // TODO
        warn!("not checking the build dependency of `{dependency}` version {version}");
    }

    // TODO: Generate this from `recipe.install`.
    let mut copies = Vec::new();

    let commands = generate_commands(
        recipe.build_data(),
        &image,
        build_root,
        &working_directory,
        target_directories,
        &mut copies,
    );

    // TODO: Take the sandbox as a parameter.
    let image = build_in_sandbox(image, commands, working_directory, copies, Sandbox::None)?;

    info!("built {}", recipe.name());

    Ok(image)
}

fn generate_commands(
    build: &Build,
    image: &HostPath,
    build_root: BuildRoot,
    working_directory: &BuildWorkingDirectory,
    target_directories: &TargetDirectories,
    copies: &mut Vec<FileTransfer>,
) -> Vec<Command> {
    let BuildRoot(build_root) = build_root;
    let BuildWorkingDirectory(working_directory) = working_directory;

    let mut commands = Vec::new();

    match &build.system {
        BuildSystem::None => (),
        // TODO: Should we add the version requirement here?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo {
            binary,
            features,
            target,
        } => {
            let cargo_manifest_path = build_root.with_suffix("Cargo.toml");
            let cargo_target_directory = working_directory.with_suffix("target");

            let mut cargo = Command::new("cargo");
            cargo
                .arg("build")
                .arg("--bin")
                .arg(&**binary)
                .arg("--locked")
                .arg("--release")
                .arg("--manifest-path")
                .arg(&*cargo_manifest_path)
                .arg("--target-dir")
                .arg(&*cargo_target_directory);

            if !features.is_empty() {
                cargo.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                cargo.arg("--target").arg(&**target);
            }

            let artefact_path = cargo_target_directory
                .with_suffix("release")
                .with_suffix(&**binary);
            let artefact_target_path = target_directories.executables().with_suffix(&**binary);

            copies.push(FileTransfer {
                from: artefact_path,
                to: artefact_target_path,
            });

            commands.push(cargo);
        }
        BuildSystem::ConfigureMake { configure_flags } => {
            let cpu_count = num_cpus::get();

            let mut configure = Command::new(&*build_root.with_suffix("configure"));

            configure.arg(flag("prefix", target_directories.prefix()));
            configure.arg(flag("bindir", target_directories.executables()));
            configure.arg(flag("sbindir", target_directories.system_executables()));
            configure.arg(flag(
                "libexecdir",
                target_directories.internal_executables(),
            ));
            configure.arg(flag("datarootdir", target_directories.data()));
            configure.arg(flag("datadir", target_directories.data()));
            configure.arg(flag("sysconfdir", target_directories.configuration()));
            configure.arg(flag("sharedstatedir", target_directories.state()));
            configure.arg(flag("localstatedir", target_directories.state()));
            configure.arg(flag("runstatedir", target_directories.runtime()));
            configure.arg(flag("includedir", target_directories.headers()));
            configure.arg(flag("libdir", target_directories.libraries()));

            for flag in configure_flags {
                configure.arg(&**flag);
            }

            let mut compile = Command::new("make");
            compile.arg(format!("-j{cpu_count}"));

            let mut install = Command::new("make");
            install
                .arg("install")
                .env(CONFIGURE_MAKE_DISTINATION_DIRECTORY, image);

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

fn flag(name: &str, path: &TargetPath) -> OsString {
    let path = path.to_os_str();

    let mut buffer = OsString::with_capacity(2 + name.len() + 1 + path.len());

    buffer.push("--");
    buffer.push(name);
    buffer.push("=");
    buffer.push(path);

    buffer
}

fn build_in_sandbox(
    image: Box<HostPath>,
    mut commands: Vec<Command>,
    working_directory: BuildWorkingDirectory,
    copies: Vec<FileTransfer>,
    sandbox: Sandbox,
) -> anyhow::Result<Image> {
    let BuildWorkingDirectory(working_directory) = working_directory;

    match sandbox {
        Sandbox::None => (),
    }

    warn!("not sand-boxing the build");

    for command in &mut commands {
        command.current_dir(&working_directory);

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

    for copy in copies {
        let destination = copy.to.with_root(&image);
        let destination: &Path = (*destination).as_ref();

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(copy.from, destination)?;
    }

    Ok(Image(image))
}
