use crate::Ledger;
use crate::recipe::Recipe;
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
    for file in &ledger.files {
        let path = target.join(file);

        let mut file = File::open(path)?;

        let mut magic_number_buffer = [0; SELFMAG];

        file.read_exact(&mut magic_number_buffer)?;

        if magic_number_buffer != *ELFMAG {
            continue;
        }

        let mut full_buffer = Vec::from(magic_number_buffer);

        file.read_to_end(&mut full_buffer)?;

        let elf = Elf::parse(&full_buffer)?;

        for library in elf.libraries {
            warn!("{} requires {}", recipe.name, library);
        }

        if let Some(library) = elf.soname {
            warn!("{} provides {}", recipe.name, library)
        }
    }

    Ok(())
}
