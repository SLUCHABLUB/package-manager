mod commands;
mod sandbox;

pub(crate) use sandbox::Sandbox;

use crate::BuildRoot;
use crate::BuildWorkingDirectory;
use crate::HostDirectories;
use crate::HostPath;
use crate::Image;
use crate::Recipe;
use crate::Source;
use crate::TargetDirectories;
use crate::build::commands::generate_commands;
use crate::build::sandbox::landlock_current_thread;
use crate::recipe::FileTransfer;
use crate::recipe::ResolvedFileTransfer;
use crate::result::convert_thread_error;
use anyhow::Context;
use anyhow::bail;
use bstr::ByteSlice;
use fn_error_context::context;
use fs_err as fs;
use fs_err::create_dir_all;
use std::path::Path;
use std::process::Command;
use std::thread;
use tracing::info;
use tracing::warn;

#[context("building {recipe}")]
pub(crate) fn build<'dependencies, Items>(
    recipe: &Recipe,
    source: Source,
    dependencies: Items,
    sandbox: Sandbox,
    target_directories: &TargetDirectories,
    host: &HostDirectories,
) -> anyhow::Result<Image>
where
    Items: IntoIterator<Item = &'dependencies Image> + Send,
{
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

    let mut copies = FileTransfer::resolve_slice(
        &recipe.install_data().copies,
        &build_root,
        target_directories,
    );

    let commands = generate_commands(
        recipe.build_data(),
        &image,
        &build_root,
        &working_directory,
        target_directories,
        &mut copies,
    )?;

    let image = build_in_sandbox(
        build_root,
        image,
        commands,
        working_directory,
        &copies,
        dependencies,
        sandbox,
    )?;

    info!("built {}", recipe.name());

    Ok(image)
}

fn build_in_sandbox<'dependencies, Images>(
    build_root: BuildRoot,
    image: Box<HostPath>,
    commands: Vec<Command>,
    working_directory: BuildWorkingDirectory,
    copies: &[ResolvedFileTransfer],
    dependencies: Images,
    sandbox: Sandbox,
) -> anyhow::Result<Image>
where
    Images: IntoIterator<Item = &'dependencies Image> + Send,
{
    // TODO: Allow automatic sandbox downgrading.
    match sandbox {
        Sandbox::None => {
            warn!("not sand-boxing the build");
            build_locally(image, commands, working_directory, copies)
        }
        Sandbox::Landlock => build_landlocked(
            build_root,
            image,
            commands,
            working_directory,
            copies,
            dependencies,
        ),
    }
}

fn build_locally(
    image: Box<HostPath>,
    mut commands: Vec<Command>,
    working_directory: BuildWorkingDirectory,
    copies: &[ResolvedFileTransfer],
) -> anyhow::Result<Image> {
    let BuildWorkingDirectory(working_directory) = working_directory;

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
        let destination = copy.destination.with_root(&image);
        let destination: &Path = (*destination).as_ref();

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&copy.source, destination)?;
    }

    Ok(Image(image))
}

fn build_landlocked<'dependencies, Images>(
    build_root: BuildRoot,
    image: Box<HostPath>,
    commands: Vec<Command>,
    working_directory: BuildWorkingDirectory,
    copies: &[ResolvedFileTransfer],
    dependencies: Images,
) -> anyhow::Result<Image>
where
    Images: IntoIterator<Item = &'dependencies Image> + Send,
{
    thread::scope(|scope| {
        scope
            .spawn(move || {
                landlock_current_thread(build_root, &working_directory, dependencies, &image)?;
                build_locally(image, commands, working_directory, copies)
            })
            .join()
            .map_err(convert_thread_error)?
    })
}
