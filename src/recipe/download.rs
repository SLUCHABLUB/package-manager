use crate::RecipeDirectories;
use crate::State;
use crate::VersionRequirement;
use crate::detect_tarball_compression;
use crate::find_in_index;
use crate::resolve_commit;
use anyhow::bail;
use gix::ObjectId;
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

impl Download {
    pub(in crate::recipe) fn lock(
        &self,
        directories: &RecipeDirectories,
        state: &State,
    ) -> anyhow::Result<DownloadLock> {
        Ok(match self {
            Download::None => DownloadLock::None,
            Download::Github {
                repository,
                version,
            } => {
                let url = format!("https://github.com/{repository}.git");
                let url = Url::parse(&url)?;

                let commit = resolve_commit(&url, version, directories, state)?;

                DownloadLock::Git { url, commit }
            }
            Download::Tarball { url, compression } => {
                let Some(compression) =
                    compression.or_else(|| detect_tarball_compression(url.as_str()))
                else {
                    bail!("could not detect compression of tarball at `{url}`");
                };

                DownloadLock::Tarball {
                    url: url.clone(),
                    compression,
                }
            }
            Download::TarballIndex {
                url,
                version,
                file_name_prefix,
            } => {
                let (real_url, compression) = find_in_index(url, version, file_name_prefix)?;

                // TODO: Figure out the "virtual" url and lock that one.

                DownloadLock::Tarball {
                    url: real_url,
                    compression,
                }
            }
        })
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Compression {
    None,
    Xz,
}

impl Compression {
    pub(crate) fn from_extension(extension: Option<&str>) -> Option<Compression> {
        Some(match extension {
            None => Compression::None,
            Some("xz") => Compression::Xz,
            _ => return None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum DownloadLock {
    None,
    Git { url: Url, commit: ObjectId },
    Tarball { url: Url, compression: Compression },
}
