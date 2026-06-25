use std::fmt::Display;
use tracing::error;

pub trait ResultExtension {
    type T;

    fn ok_or_log(self) -> Option<Self::T>;
}

impl<T, E> ResultExtension for Result<T, E>
where
    E: Display,
{
    type T = T;

    fn ok_or_log(self) -> Option<Self::T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                error!("{error:#}");
                None
            }
        }
    }
}
