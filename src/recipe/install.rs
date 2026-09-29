use crate::BuildRoot;
use crate::HostPath;
use crate::RelativePath;
use crate::TargetDirectories;
use crate::TargetPath;
use serde::Deserialize;
use serde::Serialize;

#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct Install {
    #[serde(rename = "copy")]
    pub copies: Box<[FileTransfer]>,
}

#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct FileTransfer {
    #[serde(rename = "from")]
    pub source: Box<RelativePath>,
    #[serde(rename = "to")]
    pub destination: TargetPathEnum,
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
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize)]
pub struct TargetPathEnum {
    directory: TopLevelTargetDirectory,
    path: Box<RelativePath>,
}

impl TargetPathEnum {
    pub(crate) fn executable(path: Box<RelativePath>) -> TargetPathEnum {
        TargetPathEnum {
            directory: TopLevelTargetDirectory::Executables,
            path,
        }
    }

    pub(crate) fn resolve(&self, target: &TargetDirectories) -> Box<TargetPath> {
        self.path.relative_to_target(self.directory.resolve(target))
    }
}

#[derive(Copy, Clone, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopLevelTargetDirectory {
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

impl TopLevelTargetDirectory {
    fn resolve(self, target: &TargetDirectories) -> &TargetPath {
        match self {
            TopLevelTargetDirectory::Configuration => target.configuration(),
            TopLevelTargetDirectory::Data => target.data(),
            TopLevelTargetDirectory::Executables => target.executables(),
            TopLevelTargetDirectory::Headers => target.headers(),
            TopLevelTargetDirectory::InternalExecutables => target.internal_executables(),
            TopLevelTargetDirectory::Libraries => target.libraries(),
            TopLevelTargetDirectory::Runtime => target.runtime(),
            TopLevelTargetDirectory::State => target.state(),
            TopLevelTargetDirectory::SystemExecutables => target.system_executables(),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedFileTransfer {
    pub source: Box<HostPath>,
    pub destination: Box<TargetPath>,
}
