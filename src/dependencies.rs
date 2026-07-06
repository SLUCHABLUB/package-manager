use crate::Ledger;
use crate::Recipe;
use crate::ResultExtension as _;
use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use anyhow::bail;
use fn_error_context::context;
use fs_err::File;
use goblin::elf::Elf;
use goblin::elf64::header::ELFMAG;
use goblin::elf64::header::SELFMAG;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

#[context("checking the runtime dependencies for the built `{}` recipe", recipe.name)]
pub(crate) fn check_runtime_dependencies(
    ledger: &Ledger,
    target: &Path,
    recipe: &Recipe,
) -> anyhow::Result<()> {
    let elves: Vec<_> = ledger
        .files
        .iter()
        .filter_map(|target_relative_path| {
            let absolute_path = target.join(target_relative_path);

            parse_node(&absolute_path, target_relative_path.as_os_str())
        })
        .collect();

    let internal_provides: HashMap<&str, &Version> = elves
        .iter()
        .filter_map(|elf| {
            elf.provides
                .as_ref()
                .map(|(package, version)| (&**package, version))
        })
        .collect();

    for elf in &elves {
        for (name, needed_version) in &elf.needs {
            let name = &**name;

            if let Some(internally_provided_version) = internal_provides.get(name)
                && internally_provided_version.satisfies(needed_version)
            {
                continue;
            }

            let Some(declared_version) = recipe.dependencies.versions.get(name) else {
                bail!(
                    "the file `{}` requires the library `{name}` with version {needed_version} but it was not declared as a dependency",
                    elf.file_name.display(),
                );
            };

            if !declared_version.always_satisfies(needed_version) {
                bail!(
                    "the file `{}` requires the library ``{name}` with version {needed_version} but the declared dependency has version {declared_version}",
                    elf.file_name.display(),
                )
            }
        }
    }

    Ok(())
}

struct ElfNode<'ledger> {
    file_name: &'ledger OsStr,
    provides: Option<(Box<str>, Version)>,
    needs: HashMap<Box<str>, VersionRequirement>,
}

fn parse_node<'ledger>(path: &Path, file_name: &'ledger OsStr) -> Option<ElfNode<'ledger>> {
    let mut file = File::open(path).ok_or_log()?;

    let mut magic_number_buffer = [0; SELFMAG];

    file.read_exact(&mut magic_number_buffer).ok_or_log()?;

    if magic_number_buffer != *ELFMAG {
        return None;
    }

    let mut full_buffer = Vec::from(magic_number_buffer);

    file.read_to_end(&mut full_buffer).ok_or_log()?;

    let elf = Elf::parse(&full_buffer).ok_or_log()?;

    Some(ElfNode {
        file_name,
        provides: elf
            .soname
            .and_then(|so_name| parse_so_provision(so_name).ok_or_log()),
        needs: elf
            .libraries
            .into_iter()
            .filter_map(|so_name| parse_so_requirement(so_name).ok_or_log())
            .collect(),
    })
}

fn parse_so_requirement(file_name: &str) -> anyhow::Result<(Box<str>, VersionRequirement)> {
    let (name, version) = parse_so_provision(file_name)?;

    let requirement = if version.is_empty() {
        VersionRequirement::Any
    } else {
        VersionRequirement::Exact(version)
    };

    Ok((name, requirement))
}

#[context("parsing the shared object file name")]
fn parse_so_provision(file_name: &str) -> anyhow::Result<(Box<str>, Version)> {
    let (name, suffix) = file_name
        .split_once(".so")
        .context("splitting on the `.so` extension")?;

    let name = Box::from(name);

    if suffix.is_empty() {
        return Ok((name, Version::empty()));
    }

    let Some(version) = suffix.strip_prefix(".") else {
        bail!("the version suffix does not start with a `.`")
    };

    Ok((name, Version::from(version)))
}
