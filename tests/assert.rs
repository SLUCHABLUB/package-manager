#[allow(unused)]
pub(crate) trait ResultExtension {
    type T;
    type E;

    fn assert_ok(self) -> Self::T;
}

impl<T, E> ResultExtension for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    type T = T;
    type E = E;

    fn assert_ok(self) -> T {
        match self {
            Ok(ok) => ok,
            Err(error) => panic!("{:#}", error.into()),
        }
    }
}
