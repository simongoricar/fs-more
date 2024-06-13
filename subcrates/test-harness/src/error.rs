use thiserror::Error;

/// Main `Error` for use in unit and integration tests.
///
/// Implements `From` for:
/// - [`std::io::Error`].
#[derive(Error, Debug)]
pub enum TestError {
    #[error("std::io::Error")]
    IoError(
        #[from]
        #[source]
        std::io::Error,
    ),
}

/// A main `Result` type for use in unit and integration tests (shorthand for the [`TestError`] error).
pub type TestResult<O = ()> = std::result::Result<O, TestError>;
