use anyhow::Context as _;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "Representation", into = "Representation")]
pub enum Version {
    /// A simple [Semantic Version](https://semver.org).
    /// Prereleases and build metadata are not supported.
    /// If you need that, you may use a [non-semantic version](Version::NonSemantic).
    Semantic {
        major: u64,
        minor: u64,
        patch: u64,
    },
    NonSemantic(Box<str>),
    Any,
}

impl Version {
    pub fn satisfies(&self, other: &Version) -> bool {
        if *other == Version::Any {
            return true;
        }

        match self {
            // Even patch releases may have breaking changes for unstable software.
            Version::Semantic {
                major: 0,
                minor: _,
                patch: _,
            } => self == other,
            Version::Semantic {
                major: self_major,
                minor: self_minor,
                patch: self_patch,
            } => {
                let Version::Semantic {
                    major: other_major,
                    minor: other_minor,
                    patch: other_patch,
                } = other
                else {
                    return false;
                };

                // The patch check is technically unnecessary since a patch cannot introduce new functionality upon which someone can rely.
                // However, it can be nice to specify a patch version to make sure that you get some certain bug fix.
                // This is optional to do however.
                self_major == other_major
                    && (self_minor > other_minor
                        || self_minor == other_minor && self_patch >= other_patch)
            }
            _ => self == other,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (
                Version::Semantic {
                    major: self_major,
                    minor: self_minor,
                    patch: self_patch,
                },
                Version::Semantic {
                    major: other_major,
                    minor: other_minor,
                    patch: other_patch,
                },
            ) => PartialOrd::partial_cmp(
                &(self_major, self_minor, self_patch),
                &(other_major, other_minor, other_patch),
            ),
            (Version::NonSemantic(self_version), Version::NonSemantic(other_version)) => {
                (self_version == other_version).then_some(Ordering::Equal)
            }
            (Version::Any, Version::Any) => Some(Ordering::Equal),
            _ => None,
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Semantic {
                major,
                minor,
                patch,
            } => write!(f, "{major}.{minor}.{patch}"),
            Version::NonSemantic(version) => write!(f, "{version}"),
            Version::Any => write!(f, "*"),
        }
    }
}

impl From<&str> for Version {
    fn from(string: &str) -> Version {
        if string == "*" {
            return Version::Any;
        }

        if let Ok([major, minor, patch]) = parse_semantic(string) {
            Version::Semantic {
                major,
                minor,
                patch,
            }
        } else {
            Version::NonSemantic(Box::from(string))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Representation {
    Semantic(Cow<'static, str>),
    NonSemantic { non_semantic: Box<str> },
}
impl TryFrom<Representation> for Version {
    type Error = anyhow::Error;

    fn try_from(representation: Representation) -> Result<Self, Self::Error> {
        Ok(match representation {
            Representation::Semantic(string) => {
                if string == "*" {
                    return Ok(Version::Any);
                }

                let [major, minor, patch] = parse_semantic(&string)?;

                Version::Semantic {
                    major,
                    minor,
                    patch,
                }
            }
            Representation::NonSemantic { non_semantic } => Version::NonSemantic(non_semantic),
        })
    }
}

impl From<Version> for Representation {
    fn from(version: Version) -> Self {
        match version {
            Version::Semantic {
                major,
                minor,
                patch,
            } => Representation::Semantic(Cow::Owned(format!("{major}.{minor}.{patch}"))),
            Version::NonSemantic(string) => Representation::NonSemantic {
                non_semantic: string,
            },
            Version::Any => Representation::Semantic(Cow::Borrowed("*")),
        }
    }
}

fn parse_semantic(string: &str) -> anyhow::Result<[u64; 3]> {
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

    Ok([major, minor, patch])
}
