use serde::Deserialize;
use serde::Serialize;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::Display;
use std::mem::transmute;
use std::ops::Deref;
use std::path::MAIN_SEPARATOR_STR;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[repr(transparent)]
struct AbsolutePath(Path);

impl AbsolutePath {
    fn new_unchecked(path: &Path) -> &AbsolutePath {
        // SAFETY: `AbsolutePath` is `repr(transparent)`
        unsafe { &*(std::ptr::from_ref(path) as *const AbsolutePath) }
    }

    fn new_boxed_unchecked(path: Box<Path>) -> Box<AbsolutePath> {
        // SAFETY: `AbsolutePath` is `repr(transparent)`
        unsafe { transmute(path) }
    }

    fn new(path: &Path) -> Option<&AbsolutePath> {
        path.is_absolute().then(|| Self::new_unchecked(path))
    }

    fn new_boxed(path: Box<Path>) -> Result<Box<AbsolutePath>, Box<Path>> {
        if path.is_absolute() {
            Ok(Self::new_boxed_unchecked(path))
        } else {
            Err(path)
        }
    }

    // TODO: Add some, "with_suffixes" variant to add multiple suffixes at once.
    /// Joins a relative path onto the end of this path.
    /// The suffix is assumed to be relative.
    fn with_suffix(&self, suffix: &Path) -> Box<AbsolutePath> {
        let Self(prefix) = self;

        debug_assert!(suffix.is_relative());

        // TODO: We should use `SEPARATORS_STR` since some weird OSes may have multiple separators.
        let separator_size = usize::from(!prefix.ends_with(MAIN_SEPARATOR_STR));

        let mut buffer = PathBuf::with_capacity(
            prefix.as_os_str().len() + separator_size + suffix.as_os_str().len(),
        );

        buffer.as_mut_os_string().push(prefix.as_os_str());
        buffer.push(suffix);

        debug_assert_eq!(
            buffer.as_os_str().len(),
            prefix.as_os_str().len() + separator_size + suffix.as_os_str().len(),
            "we estimated the capacity wrong",
        );

        debug_assert!(buffer.is_absolute());

        Self::new_boxed_unchecked(Box::<Path>::from(buffer))
    }

    fn to_relative(&self) -> &Path {
        let Self(path) = self;
        // TODO: This may panic on some esoteric OSes.
        path.strip_prefix(MAIN_SEPARATOR_STR)
            .expect(const_str::format!(
                "an absolute path should start with `{MAIN_SEPARATOR_STR}`"
            ))
    }
}

impl Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let AbsolutePath(path) = self;
        write!(f, "{}", path.display())
    }
}

#[derive(Debug, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct HostPath(AbsolutePath);

impl HostPath {
    fn from_absolute(path: &AbsolutePath) -> &HostPath {
        // SAFETY: `HostPath` is `repr(transparent)`
        unsafe { &*(std::ptr::from_ref(path) as *const HostPath) }
    }

    fn from_absolute_boxed(path: Box<AbsolutePath>) -> Box<HostPath> {
        // SAFETY: `HostPath` is `repr(transparent)`
        unsafe { transmute(path) }
    }

    pub(crate) fn new(path: &Path) -> Option<&HostPath> {
        AbsolutePath::new(path).map(Self::from_absolute)
    }

    pub(crate) fn new_boxed(path: Box<Path>) -> Result<Box<HostPath>, Box<Path>> {
        AbsolutePath::new_boxed(path).map(Self::from_absolute_boxed)
    }

    /// Joins a relative path onto the end of this path.
    /// The suffix is assumed to be relative.
    pub(crate) fn with_suffix<Suffix>(&self, suffix: Suffix) -> Box<HostPath>
    where
        Suffix: AsRef<Path>,
    {
        let HostPath(prefix) = self;
        HostPath::from_absolute_boxed(prefix.with_suffix(suffix.as_ref()))
    }
}

impl Display for HostPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let HostPath(path) = self;
        write!(f, "{path}")
    }
}

impl AsRef<OsStr> for HostPath {
    fn as_ref(&self) -> &OsStr {
        let HostPath(AbsolutePath(path)) = self;
        path.as_os_str()
    }
}

impl AsRef<Path> for HostPath {
    fn as_ref(&self) -> &Path {
        let HostPath(AbsolutePath(path)) = self;
        path
    }
}

impl AsRef<Path> for Box<HostPath> {
    fn as_ref(&self) -> &Path {
        let HostPath(AbsolutePath(path)) = self.as_ref();
        path
    }
}

impl Deref for HostPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        let HostPath(AbsolutePath(path)) = self;
        path
    }
}

impl<'de> Deserialize<'de> for Box<HostPath> {
    fn deserialize<D>(deserialiser: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Box::<Path>::deserialize(deserialiser).and_then(|path| {
            HostPath::new_boxed(path).map_err(|path| {
                serde::de::Error::custom(format!("the path `{}` is not absolute", path.display()))
            })
        })
    }
}

#[derive(Debug, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct TargetPath(AbsolutePath);

impl TargetPath {
    fn from_absolute(path: &AbsolutePath) -> &TargetPath {
        // SAFETY: `TargetPath is `repr(transparent)`
        unsafe { &*(std::ptr::from_ref(path) as *const TargetPath) }
    }

    fn from_absolute_boxed(path: Box<AbsolutePath>) -> Box<TargetPath> {
        // SAFETY: `TargetPath` is `repr(transparent)`
        unsafe { transmute(path) }
    }

    pub(crate) fn new_boxed(path: Box<Path>) -> Result<Box<TargetPath>, Box<Path>> {
        AbsolutePath::new_boxed(path).map(Self::from_absolute_boxed)
    }

    // TODO: Returning a `Box` here is a skill issue.
    pub(crate) fn from_path_and_root(path: &HostPath, root: &HostPath) -> Box<TargetPath> {
        let HostPath(AbsolutePath(path)) = path;
        let HostPath(AbsolutePath(root)) = root;

        // This takes into account `root` ending with `/`.
        let relative = path
            .strip_prefix(root)
            .expect("path should start with root");

        let root = TargetPath::from_absolute(AbsolutePath::new_unchecked(Path::new("/")));

        root.with_suffix(relative)
    }

    // This is deliberately not `AsRef` to avoid accidental misuse.
    pub(crate) fn to_os_str(&self) -> &OsStr {
        let TargetPath(AbsolutePath(path)) = self;

        path.as_os_str()
    }

    /// Joins a relative path onto the end of this path.
    /// The suffix is assumed to be relative.
    pub(crate) fn with_suffix<Suffix>(&self, suffix: Suffix) -> Box<TargetPath>
    where
        Suffix: AsRef<Path>,
    {
        let TargetPath(prefix) = self;
        TargetPath::from_absolute_boxed(prefix.with_suffix(suffix.as_ref()))
    }

    pub(crate) fn with_root(&self, root: &HostPath) -> Box<HostPath> {
        let TargetPath(path) = self;
        root.with_suffix(path.to_relative())
    }
}

impl Clone for Box<TargetPath> {
    fn clone(&self) -> Self {
        let TargetPath(AbsolutePath(path)) = &**self;
        let path = Box::<Path>::from(path);
        TargetPath::new_boxed(path).expect("the path should be absolute")
    }
}

impl Display for TargetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let TargetPath(path) = self;
        write!(f, "{path}")
    }
}

impl<'de> Deserialize<'de> for Box<TargetPath> {
    fn deserialize<D>(deserialiser: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Box::<Path>::deserialize(deserialiser).and_then(|path| {
            TargetPath::new_boxed(path).map_err(|path| {
                serde::de::Error::custom(format!("the path `{}` is not absolute", path.display()))
            })
        })
    }
}
