use crate::HostDirectories;
use crate::HostPath;
use crate::IndexedFile;
use crate::Version;
use crate::detect_tarball_compression;
use crate::find_in_index;
use crate::recipe::find_cached_repository_or_initialise;
use crate::resolve_commit;
use anyhow::bail;
use fs_err as fs;
use gix::ObjectId;
use gix::Repository;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

#[derive(Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Download {
    None,
    Github {
        repository: Box<str>,
    },
    Tarball {
        url: Url,
        compression: Option<Compression>,
    },
    TarballIndex {
        url: Url,
        file_name_prefix: Box<str>,
    },
}

impl Download {
    pub(crate) fn lock(
        &self,
        host: &HostDirectories,
        version: &Version,
    ) -> anyhow::Result<DownloadLock> {
        Ok(match self {
            Download::None => DownloadLock::None,
            Download::Github { repository } => {
                let url = format!("https://github.com/{repository}.git");
                let url = Url::parse(&url)?;

                let repository = find_cached_repository_or_initialise(&url, host)?;

                let commit = resolve_commit(&repository, &url, version)?;

                DownloadLock::Git {
                    repository: Box::new(repository),
                    url,
                    commit,
                }
            }
            Download::Tarball { url, compression } => {
                let Some(compression) =
                    compression.or_else(|| detect_tarball_compression(url.as_str()))
                else {
                    bail!("could not detect compression of tarball at `{url}`");
                };

                DownloadLock::Tarball {
                    real_url: url.clone(),
                    virtual_url: None,
                    compression,
                }
            }
            Download::TarballIndex {
                url,
                file_name_prefix,
            } => {
                let IndexedFile {
                    real_url,
                    virtual_url,
                    compression,
                } = find_in_index(url, version, file_name_prefix)?;

                DownloadLock::Tarball {
                    real_url,
                    virtual_url,
                    compression,
                }
            }
        })
    }
}

#[derive(Copy, Clone, Hash, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DownloadLock {
    #[serde(skip)]
    None,
    Git {
        url: Url,
        commit: ObjectId,
        // Boxed so clippy doesn't complain about it's size.
        #[serde(skip_serializing)]
        repository: Box<Repository>,
    },
    Tarball {
        real_url: Url,
        virtual_url: Option<Url>,
        compression: Compression,
    },
}

impl DownloadLock {
    pub(crate) fn read_from(
        path: &HostPath,
        host: &HostDirectories,
    ) -> anyhow::Result<DownloadLock> {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum DownloadLockSerial {
            Git {
                url: Url,
                commit: ObjectId,
            },
            Tarball {
                real_url: Url,
                virtual_url: Option<Url>,
                compression: Compression,
            },
        }

        let file_contents = fs::read_to_string(path)?;
        let serial = toml::from_str::<DownloadLockSerial>(&file_contents)?;

        Ok(match serial {
            DownloadLockSerial::Git { url, commit } => DownloadLock::Git {
                repository: Box::new(find_cached_repository_or_initialise(&url, host)?),
                url,
                commit,
            },

            DownloadLockSerial::Tarball {
                real_url,
                virtual_url,
                compression,
            } => DownloadLock::Tarball {
                real_url,
                virtual_url,
                compression,
            },
        })
    }

    pub(crate) fn write_to(&self, path: &HostPath) -> anyhow::Result<()> {
        if matches!(self, DownloadLock::None) {
            return Ok(());
        }

        let serialised = toml::to_string(self)?;
        fs::write(path, serialised.as_bytes())?;

        Ok(())
    }
}
