use std::backtrace::Backtrace;
use std::fmt::Display;

#[derive(Debug)]
pub(crate) struct InternalError {
    pub error: anyhow::Error,
    pub backtrace: Backtrace,
}

impl Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "INTERNAL ERROR: {:#}\n\nPlease file a bug report at: {}\n\nBacktrace:\n\n{}",
            self.error,
            concat!(env!("CARGO_PKG_REPOSITORY"), "/issues"),
            self.backtrace,
        )
    }
}

macro_rules! bail_internal {
    ($($token:tt)*) => {
        ::anyhow::bail!($crate::internal_error::InternalError {
            error: ::anyhow::anyhow!($($token)*),
            backtrace: ::std::backtrace::Backtrace::force_capture(),
        })
    };
}

pub(crate) use bail_internal;
