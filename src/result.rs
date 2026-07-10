use once_cell::race::OnceBool;
use std::any::TypeId;
use std::env;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fmt::Display;
use tracing::error;

pub(crate) trait ResultExtension {
    type T;

    fn ok_or_log(self) -> Option<Self::T>;
}

impl<T, E> ResultExtension for Result<T, E>
where
    E: Debug + Display + 'static,
{
    type T = T;

    fn ok_or_log(self) -> Option<Self::T> {
        static BACKTRACE: OnceBool = OnceBool::new();

        let backtrace = BACKTRACE.get_or_init(|| match env::var_os("RUST_LIB_BACKTRACE") {
            Some(string) if string == OsStr::new("0") => false,
            Some(_) => true,
            None => match env::var_os("RUST_BACKTRACE") {
                Some(string) if string == OsStr::new("0") => false,
                Some(_) => true,
                None => false,
            },
        });

        let is_anyhow = TypeId::of::<E>() == TypeId::of::<anyhow::Error>();

        match self {
            Ok(value) => Some(value),
            Err(error) if is_anyhow && backtrace => {
                error!("{error:?}");
                None
            }
            Err(error) => {
                error!("{error:#}");
                None
            }
        }
    }
}
