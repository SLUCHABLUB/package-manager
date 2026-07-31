use crate::HostDirectories;
use crate::Image;
use crate::ImageLedger;
use crate::Recipe;
use crate::SystemLedger;
use crate::TargetDirectories;
use anyhow::Context;
use fn_error_context::context;
use fs_err as fs;
use fs_err::remove_dir_all;
use std::io;
use tracing::info;

pub(crate) fn stage(
    packages: impl IntoIterator<Item = (&'static Recipe, Image, ImageLedger)>,
    host: &HostDirectories,
    target_directories: &TargetDirectories,
) -> anyhow::Result<SystemLedger> {
    info!("staging the packages");

    let staging = &host.staging;

    match remove_dir_all(staging) {
        Ok(()) => (),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        result @ Err(_) => result?,
    }

    let mut system_ledger = SystemLedger::new(target_directories);

    for (recipe, image, image_ledger) in packages {
        stage_single(recipe, image, image_ledger, host, &mut system_ledger)?;
    }

    system_ledger.write_to_root(staging)?;

    info!("staging complete");

    Ok(system_ledger)
}

#[context("staging {recipe}")]
fn stage_single(
    recipe: &Recipe,
    image: Image,
    image_ledger: ImageLedger,
    host: &HostDirectories,
    system_ledger: &mut SystemLedger,
) -> anyhow::Result<()> {
    info!("staging {recipe}");

    let Image(image) = image;

    for (entry, _hash) in image_ledger.files() {
        let source = entry.with_root(&image);
        let destination = entry.with_root(&host.staging);

        let destination_parent = destination
            .parent()
            .with_context(|| format!("getting the parent of `{destination}`"))?;

        // TODO: Directory permissions?
        fs::create_dir_all(destination_parent)?;
        fs::copy(source, destination)?;
    }

    system_ledger.add_image(image_ledger);

    Ok(())
}
