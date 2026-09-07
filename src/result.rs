use std::any::Any;
use std::env;
use std::ffi::OsStr;
use std::mem::forget;
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

pub(crate) fn convert_result<T, E>(result: Result<T, E>) -> anyhow::Result<T>
where
    E: Into<anyhow::Error> + 'static,
{
    result.map_err(E::into)
}

pub(crate) fn convert_thread_error(error: Box<dyn Any + Send + 'static>) -> anyhow::Error {
    let error_message = thread_error_to_error_message(&error);
    let new_error = anyhow::Error::msg(error_message.to_owned());

    // Dropping this might panic.
    forget(error);

    new_error
}

fn thread_error_to_error_message<'error>(error: &'error (dyn Any + Send + 'static)) -> &'error str {
    if let Some(message) = error.downcast_ref::<&'static str>() {
        message
    } else if let Some(message) = error.downcast_ref::<String>() {
        message
    } else {
        "unknown error"
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
