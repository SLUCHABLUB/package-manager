use crate::Ledger;
use crate::Version;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::bail;
use fs_err::File;
use goblin::elf::Elf;
use goblin::elf64::header::ELFMAG;
use goblin::elf64::header::SELFMAG;
use std::io::Read;
use std::path::Path;
use tracing::warn;

pub fn check_runtime_dependencies(
    ledger: &Ledger,
    target: &Path,
    recipe: &Recipe,
) -> anyhow::Result<()> {
    for target_relative_path in &ledger.files {
        let absolute_path = target.join(target_relative_path);

        let mut file = File::open(absolute_path)?;

        let mut magic_number_buffer = [0; SELFMAG];

        file.read_exact(&mut magic_number_buffer)?;

        if magic_number_buffer != *ELFMAG {
            continue;
        }

        let mut full_buffer = Vec::from(magic_number_buffer);

        file.read_to_end(&mut full_buffer)?;

        let elf = Elf::parse(&full_buffer)?;

        for library in elf.libraries {
            let (name, needed_version) = parse_so_name(library)
                .with_context(|| format!("parsing {library} as a shared object file name"))?;

            let Some(declared_version) = recipe.dependencies.versions.get(name) else {
                bail!(
                    "the file `{}` requires the library `{library}` but it was not declared as a dependency of `{}`",
                    target_relative_path.display(),
                    &recipe.name
                );
            };

            if !declared_version.satisfies(&needed_version) {
                bail!(
                    "the file `{}` requires the library `{library}` but the declared dependency of `{}` has version `{declared_version}`",
                    target_relative_path.display(),
                    recipe.name,
                )
            }
        }

        if let Some(library) = elf.soname {
            warn!("{} provides {}", recipe.name, library)
        }
    }

    Ok(())
}

fn parse_so_name(name: &str) -> anyhow::Result<(&str, Version)> {
    let (name, suffix) = name
        .split_once(".so")
        .context("splitting on the `.so` extension")?;

    if suffix.is_empty() {
        warn!("NEEDED shared object `{name}` is unversioned");
        return Ok((name, Version::Any));
    }

    let Some(version) = suffix.strip_prefix(".") else {
        bail!("the version suffix does not start with a `.`")
    };

    Ok((name, Version::from(version)))
}
