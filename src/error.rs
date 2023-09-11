use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // TODO
}

#[derive(Error, Debug)]
pub enum FileError {
    #[error("source file does not exist")]
    NotFound,

    #[error("provided source file path is not a file")]
    NotAFile,

    #[error("unable to access source file")]
    UnableToAccessSourceFile { error: std::io::Error },

    #[error("target file already exists")]
    AlreadyExists,

    #[error("unable to access target file")]
    UnableToAccessTargetFile { error: std::io::Error },

    #[error("unable to canonicalize source file path")]
    UnableToCanonicalizeSourcePath { error: std::io::Error },

    #[error("unable to canonicalize target file path")]
    UnableToCanonicalizeTargetPath { error: std::io::Error },

    #[error("source and target file path are the same file")]
    SourceAndTargetAreTheSameFile,

    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },

    #[error("other conversion error: {error}")]
    OtherConversionError { error: core::num::TryFromIntError },
}

#[derive(Error, Debug)]
pub enum FileRemoveError {
    // TODO
}
