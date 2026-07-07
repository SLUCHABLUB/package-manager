use crate::SemanticVersion;
use crate::Version;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr as _;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VersionRequirement {
    Exact(Version),
    Semantic(SemanticVersion),
    Any,
}

impl VersionRequirement {
    pub(crate) fn always_satisfies(&self, requirement: &VersionRequirement) -> bool {
        match (self, requirement) {
            (_, VersionRequirement::Any) => true,
            (VersionRequirement::Exact(version), VersionRequirement::Exact(requirement)) => {
                version == requirement
            }
            (VersionRequirement::Exact(version), VersionRequirement::Semantic(requirement)) => {
                SemanticVersion::from_str(&version.string)
                    .is_ok_and(|version| version.satisfies(*requirement))
            }
            (VersionRequirement::Semantic(version), VersionRequirement::Semantic(requirement)) => {
                version.satisfies(*requirement)
            }
            _ => false,
        }
    }
}

impl Display for VersionRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionRequirement::Exact(version) => write!(f, "{version}"),
            VersionRequirement::Semantic(version) => write!(f, "{version}"),
            VersionRequirement::Any => write!(f, "\"any\""),
        }
    }
}
