use crate::BuildRoot;
use crate::HostPath;
use crate::RelativePath;
use crate::TargetDirectories;
use crate::TargetPath;
use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::de::IgnoredAny;
use serde::de::Visitor;
use serde::ser::SerializeMap;

#[derive(Hash, Debug, Default, Serialize, Deserialize)]
pub struct Install {
    #[serde(rename = "copy")]
    pub copies: Box<[FileTransfer]>,
}

#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct FileTransfer {
    #[serde(rename = "from")]
    pub source: Box<RelativePath>,
    #[serde(rename = "to")]
    pub destination: NamespacedTargetPath,
}

impl FileTransfer {
    pub(crate) fn resolve(
        &self,
        build_root: &BuildRoot,
        target: &TargetDirectories,
    ) -> ResolvedFileTransfer {
        let BuildRoot(build_root) = build_root;

        ResolvedFileTransfer {
            source: self.source.relative_to_host(build_root),
            destination: self.destination.resolve(target),
        }
    }

    pub(crate) fn resolve_slice(
        file_transfers: &[FileTransfer],
        build_root: &BuildRoot,
        target: &TargetDirectories,
    ) -> Vec<ResolvedFileTransfer> {
        file_transfers
            .iter()
            .map(|file_transfer| file_transfer.resolve(build_root, target))
            .collect()
    }
}

#[derive(Clone, Hash, Debug)]
pub struct NamespacedTargetPath {
    directory: TargetDirectoryNamespace,
    path: Box<RelativePath>,
}

impl NamespacedTargetPath {
    pub(crate) fn executable(path: Box<RelativePath>) -> NamespacedTargetPath {
        NamespacedTargetPath {
            directory: TargetDirectoryNamespace::Executables,
            path,
        }
    }

    pub(crate) fn resolve(&self, target: &TargetDirectories) -> Box<TargetPath> {
        self.path.relative_to_target(self.directory.resolve(target))
    }
}

impl Serialize for NamespacedTargetPath {
    fn serialize<S>(&self, serialiser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serialiser = serialiser.serialize_map(Some(1))?;

        serialiser.serialize_entry(&self.directory, &self.path)?;

        serialiser.end()
    }
}

impl<'de> Deserialize<'de> for NamespacedTargetPath {
    fn deserialize<D>(deserialiser: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Parser;

        impl<'de> Visitor<'de> for Parser {
            type Value = NamespacedTargetPath;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a namespaced target-path")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let (directory, path) = map
                    .next_entry()?
                    .ok_or_else(|| de::Error::custom("missing namespace"))?;

                if let Some(IgnoredAny) = map.next_key()? {
                    return Err(de::Error::custom("only one namespace expected"));
                }

                Ok(NamespacedTargetPath { directory, path })
            }
        }

        deserialiser.deserialize_map(Parser)
    }
}

#[derive(Copy, Clone, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetDirectoryNamespace {
    Configuration,
    Data,
    Executables,
    Headers,
    InternalExecutables,
    Libraries,
    Runtime,
    State,
    SystemExecutables,
}

impl TargetDirectoryNamespace {
    fn resolve(self, target: &TargetDirectories) -> &TargetPath {
        match self {
            TargetDirectoryNamespace::Configuration => target.configuration(),
            TargetDirectoryNamespace::Data => target.data(),
            TargetDirectoryNamespace::Executables => target.executables(),
            TargetDirectoryNamespace::Headers => target.headers(),
            TargetDirectoryNamespace::InternalExecutables => target.internal_executables(),
            TargetDirectoryNamespace::Libraries => target.libraries(),
            TargetDirectoryNamespace::Runtime => target.runtime(),
            TargetDirectoryNamespace::State => target.state(),
            TargetDirectoryNamespace::SystemExecutables => target.system_executables(),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedFileTransfer {
    pub source: Box<HostPath>,
    pub destination: Box<TargetPath>,
}
