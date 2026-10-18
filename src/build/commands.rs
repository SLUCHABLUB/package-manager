use crate::BuildRoot;
use crate::BuildSystem;
use crate::BuildWorkingDirectory;
use crate::HostPath;
use crate::NamespacedTargetPath;
use crate::RelativePath;
use crate::TargetDirectories;
use crate::TargetPath;
use crate::recipe::Build;
use crate::recipe::ResolvedFileTransfer;
use anyhow::Context as _;
use std::ffi::OsString;
use std::process::Command;

const CONFIGURE_MAKE_DISTINATION_DIRECTORY: &str = concat!("DEST", "DIR");

pub(super) fn generate_commands(
    build: &Build,
    image: &HostPath,
    build_root: &BuildRoot,
    working_directory: &BuildWorkingDirectory,
    target_directories: &TargetDirectories,
    copies: &mut Vec<ResolvedFileTransfer>,
) -> anyhow::Result<Vec<Command>> {
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
                .arg(cargo_manifest_path)
                .arg("--target-dir")
                .arg(&cargo_target_directory);

            if !features.is_empty() {
                cargo.arg("--features").arg(features.join(" "));
            }

            if let Some(target) = target {
                cargo.arg("--target").arg(&**target);
            }

            let artefact_path = cargo_target_directory
                .with_suffix("release")
                .with_suffix(&**binary);

            let binary_path = RelativePath::new(&**binary)
                .context("parsing crate name as a path component")?
                .to_owned();

            copies.push(ResolvedFileTransfer {
                source: artefact_path,
                destination: NamespacedTargetPath::executable(binary_path)
                    .resolve(target_directories),
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

    Ok(commands)
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
