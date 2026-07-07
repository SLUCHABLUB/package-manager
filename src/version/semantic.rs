use anyhow::Context as _;
use anyhow::bail;
use serde_with::DeserializeFromStr;
use serde_with::SerializeDisplay;
use std::fmt::Display;
use std::str::FromStr;

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, SerializeDisplay, DeserializeFromStr,
)]
pub(crate) struct SemanticVersion([u64; 3]);

impl SemanticVersion {
    pub(in crate::version) fn satisfies(self, requirement: SemanticVersion) -> bool {
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
        let mut segments = string.split('.');

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
