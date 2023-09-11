use assert_fs::fixture::FixtureError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("assert_fs' FixtureError: {0}")]
    FixtureError(#[from] FixtureError),

    #[error("std::io::Error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type TestResult<O> = std::result::Result<O, TestError>;
