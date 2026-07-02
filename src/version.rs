use crate::ResultExtension;
use anyhow::Context as _;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;
use serde_with::DeserializeFromStr;
use serde_with::SerializeDisplay;
use std::fmt::Display;
use std::str::FromStr;

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

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, SerializeDisplay, DeserializeFromStr,
)]
pub(crate) struct SemanticVersion([u64; 3]);

impl SemanticVersion {
    fn satisfies(self, requirement: SemanticVersion) -> bool {
        // Since a patch cannot introduce functionality, it's irrelevant when checking for compatibility.
        let SemanticVersion([major, minor, _]) = self;
        let SemanticVersion([major_requirement, minor_requiremnent, _]) = requirement;

        if major_requirement == 0 {
            self == requirement
        } else {
            major == major_requirement && minor >= minor_requiremnent
        }
    }
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let SemanticVersion([major, minor, patch]) = self;
        write!(f, "{major}.{minor}.{patch}")
    }
}

impl FromStr for SemanticVersion {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> anyhow::Result<SemanticVersion> {
        let mut segments = string.split(".");

        let major = segments
            .next()
            .expect("`str::split` should not yield an empty iterator");

        let major = u64::from_str(major).context("parsing the major version")?;

        let minor = match segments.next() {
            Some(segment) => u64::from_str(segment).context("parsing the minor version")?,
            None if major == 0 => bail!("missing minor version when major is 0"),
            None => 0,
        };

        let patch = match segments.next() {
            Some(segment) => u64::from_str(segment).context("parsing the patch version")?,
            None if major == 0 => bail!("missing patch version when major is 0"),
            None => 0,
        };

        if segments.next().is_some() {
            bail!("too many segments");
        }

        Ok(SemanticVersion([major, minor, patch]))
    }
}
