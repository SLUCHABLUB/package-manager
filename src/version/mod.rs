mod requirement;
mod resolver;
mod semantic;

use crate::ResultExtension;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;

pub(crate) use requirement::VersionRequirement;
pub(crate) use resolver::Resolver;
pub(crate) use semantic::SemanticVersion;

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Version {
    pub string: Box<str>,
}

impl Version {
    pub(crate) fn satisfies(&self, requirement: &VersionRequirement) -> bool {
        match requirement {
            VersionRequirement::Exact(requirement) => self == requirement,
            VersionRequirement::Semantic(requirement) => {
                let Some(version) = SemanticVersion::from_str(&self.string).ok_or_log() else {
                    return false;
                };

                version.satisfies(*requirement)
            }
            VersionRequirement::Any => true,
        }
    }

    pub(crate) fn empty() -> Version {
        Version {
            string: Box::from(""),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.string.is_empty()
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // The error type is literally `()`, so we're not loosing any information.
        version_compare::compare(&self.string, &other.string)
            .ok()?
            .ord()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}`", self.string)
    }
}

impl<IntoBoxStr> From<IntoBoxStr> for Version
where
    IntoBoxStr: Into<Box<str>>,
{
    fn from(string: IntoBoxStr) -> Self {
        Version {
            string: string.into(),
        }
    }
}
