use rapidhash::quality;
use std::hash::Hash;
use std::hash::Hasher as _;

pub(crate) fn hash(thing: impl Hash) -> u64 {
    // We use the `quality` version since it is the same one used for files.
    // `fast` seems to only me designed for maps.
    // `default()`, uses the default seed, keeping our hashes stable.
    let mut hasher = quality::RapidHasher::default();

    thing.hash(&mut hasher);

    hasher.finish()
}
