use crate::Version;
use crate::VersionRequirement;

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
        // TODO: Warn if we cannot compare versions.
        if version.satisfies(self.requirement)
            && self
                .best
                .as_ref()
                .is_none_or(|(_value, best_version)| version > *best_version)
        {
            self.best = Some((value, version));
        }
    }

    pub(crate) fn best(self) -> Option<T> {
        self.best.map(|(value, _version)| value)
    }
}
