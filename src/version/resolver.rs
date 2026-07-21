use crate::Version;
use crate::VersionRequirement;
use tracing::warn;

pub(crate) struct Resolver<'requirement, T> {
    requirement: &'requirement VersionRequirement,
    best: Option<(T, Version)>,
}

impl<T> Resolver<'_, T> {
    pub(crate) fn from_requirement(requirement: &VersionRequirement) -> Resolver<'_, T> {
        Resolver {
            requirement,
            best: None,
        }
    }

    pub(crate) fn add_option(&mut self, value: T, version: Version) {
        if version.satisfies(self.requirement)
            && self.best.as_ref().is_none_or(|(_value, best_version)| {
                if let Some(ordering) = PartialOrd::partial_cmp(&version, best_version) {
                    ordering.is_gt()
                } else {
                    warn!("could not compare the versions `{version}` and `{best_version}`");
                    false
                }
            })
        {
            self.best = Some((value, version));
        }
    }

    pub(crate) fn best(self) -> Option<T> {
        self.best.map(|(value, _version)| value)
    }
}
