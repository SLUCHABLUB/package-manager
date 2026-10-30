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
use std::ffi::OsStr;
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
    let mut commands = match &build.system {
        BuildSystem::None => Vec::new(),
        // TODO: Should we add the version requirement here?
        // TODO: --message-format json to get better logs?
        BuildSystem::Cargo {
            binary,
            features,
            target,
        } => {
            let (command, copy) = cargo(
                binary,
                features,
                target.as_deref(),
                target_directories,
                build_root,
                working_directory,
            )?;

            copies.push(copy);

            vec![command]
        }
        BuildSystem::ConfigureMake { configure_flags } => configure_make(
            configure_flags,
            image,
            target_directories,
            build_root,
            working_directory,
        )
        .into(),
    };

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

fn command<S: AsRef<OsStr>>(executable: S, working_directory: &BuildWorkingDirectory) -> Command {
    let BuildWorkingDirectory(working_directory) = working_directory;

    let mut command = Command::new(executable);

    command.env_clear();
    command.current_dir(working_directory);

    command
}

fn cargo(
    // TODO: Move these into a struct.
    binary: &str,
    features: &[Box<str>],
    target: Option<&str>,

    target_directories: &TargetDirectories,
    build_root: &BuildRoot,
    working_directory: &BuildWorkingDirectory,
) -> anyhow::Result<(Command, ResolvedFileTransfer)> {
    let mut command = command("cargo", working_directory);

    let BuildRoot(build_root) = build_root;
    let BuildWorkingDirectory(working_directory) = working_directory;

    let cargo_target_directory = working_directory.with_suffix("target");
    let cargo_manifest_path = build_root.with_suffix("Cargo.toml");

    command
        .arg("build")
        .arg("--bin")
        .arg(binary)
        .arg("--locked")
        .arg("--release")
        .arg("--manifest-path")
        .arg(cargo_manifest_path)
        .arg("--target-dir")
        .arg(&cargo_target_directory);

    if !features.is_empty() {
        command.arg("--features").arg(features.join(" "));
    }

    if let Some(target) = target {
        command.arg("--target").arg(target);
    }

    let artefact_path = cargo_target_directory
        .with_suffix("release")
        .with_suffix(binary);

    let binary_path = RelativePath::new(binary)
        .context("parsing crate name as a path component")?
        .to_owned();

    let artefact = ResolvedFileTransfer {
        source: artefact_path,
        destination: NamespacedTargetPath::executable(binary_path).resolve(target_directories),
    };

    Ok((command, artefact))
}

fn configure_make(
    configure_flags: &[Box<str>],

    image: &HostPath,
    target: &TargetDirectories,
    build_root: &BuildRoot,
    working_directory: &BuildWorkingDirectory,
) -> [Command; 3] {
    let BuildRoot(build_root) = build_root;

    let cpu_count = num_cpus::get();

    let mut configure = command(&*build_root.with_suffix("configure"), working_directory);

    configure.arg(flag("prefix", target.prefix()));
    configure.arg(flag("bindir", target.executables()));
    configure.arg(flag("sbindir", target.system_executables()));
    configure.arg(flag("libexecdir", target.internal_executables()));
    configure.arg(flag("datarootdir", target.data()));
    configure.arg(flag("datadir", target.data()));
    configure.arg(flag("sysconfdir", target.configuration()));
    configure.arg(flag("sharedstatedir", target.state()));
    configure.arg(flag("localstatedir", target.state()));
    configure.arg(flag("runstatedir", target.runtime()));
    configure.arg(flag("includedir", target.headers()));
    configure.arg(flag("libdir", target.libraries()));

    for flag in configure_flags {
        configure.arg(&**flag);
    }

    let mut compile = command("make", working_directory);
    compile.arg(format!("-j{cpu_count}"));

    let mut install = command("make", working_directory);
    install
        .arg("install")
        .env(CONFIGURE_MAKE_DISTINATION_DIRECTORY, image);

    [configure, compile, install]
}
