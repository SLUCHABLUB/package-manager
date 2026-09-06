use std::env;
use std::ffi::OsStr;
use std::sync::LazyLock;
use tracing::error;

pub(crate) trait ResultExtension {
    type T;

    fn ok_or_log(self) -> Option<Self::T>;
}

impl<T, E> ResultExtension for Result<T, E>
where
    E: Into<anyhow::Error> + 'static,
{
    type T = T;

    fn ok_or_log(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                log_error(error);
                None
            }
        }
    }
}

#[inline]
pub(crate) fn log_error(error: impl Into<anyhow::Error> + 'static) {
    log_anyhow_error(&error.into());
}

static BACKTRACE_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    let disabled = OsStr::new("0");

    if let Some(rust_lib_backtrace) = env::var_os("RUST_LIB_BACKTRACE") {
        rust_lib_backtrace != disabled
    } else if let Some(rust_backtrace) = env::var_os("RUST_BACKTRACE") {
        rust_backtrace != disabled
    } else {
        false
    }
});

fn log_anyhow_error(error: &anyhow::Error) {
    if *BACKTRACE_ENABLED {
        error!("{error:?}");
    } else {
        error!("{error:#}");
    }
}
