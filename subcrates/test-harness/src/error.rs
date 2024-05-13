use assert_fs::fixture::FixtureError;
use thiserror::Error;

use crate::assertable::AssertableFilePathError;

/// Main `Error` for use in unit and integration tests.
///
/// Implements `From` for:
/// - [`assert_fs::FixtureError`](../../assert_fs/fixture/struct.FixtureError.html),
/// - [`std::io::Error`].
#[derive(Error, Debug)]
pub enum TestError {
    #[error("assert_fs' FixtureError")]
    FixtureError(
        #[from]
        #[source]
        FixtureError,
    ),

    #[error("assertable file path error")]
    AssertableFilePathError(
        #[from]
        #[source]
        AssertableFilePathError,
    ),

    #[error("std::io::Error")]
    IoError(
        #[from]
        #[source]
        std::io::Error,
    ),
}

/// A main `Result` type for use in unit and integration tests (shorthand for the [`TestError`] error).
pub type TestResult<O = ()> = std::result::Result<O, TestError>;
