use crate::VersionRequirement;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Download {
    #[default]
    #[serde(skip)]
    None,
    Github {
        repository: Box<str>,
        version: VersionRequirement,
    },
    Tarball {
        url: Url,
        compression: Option<Compression>,
    },
    TarballIndex {
        url: Url,
        version: VersionRequirement,
        file_name_prefix: Box<str>,
    },
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Compression {
    None,
    Xz,
}
