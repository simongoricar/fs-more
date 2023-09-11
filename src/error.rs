//! Error structures and enums.

use thiserror::Error;

/// Represents an error state when copying or moving a file.
#[derive(Error, Debug)]
pub enum FileError {
    /// The source file cannot be found.
    #[error("source file does not exist")]
    NotFound,

    /// The source file path is not a file.
    #[error("provided source file path is not a file")]
    NotAFile,

    /// The source file cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access source file")]
    UnableToAccessSourceFile { error: std::io::Error },

    /// The target file already exists. Some copy/move options disable this check.
    #[error("target file already exists")]
    AlreadyExists,

    /// The target file cannot be accessed or written to (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access target file")]
    UnableToAccessTargetFile { error: std::io::Error },

    /// The source file path cannot be canonicalized (due to previous checks this should be very rare).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to canonicalize source file path")]
    UnableToCanonicalizeSourcePath { error: std::io::Error },

    /// The target file path cannot be canonicalized (due to previous checks this should be very rare).
    /// This can only happen *if* the target file already exists.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to canonicalize target file path")]
    UnableToCanonicalizeTargetPath { error: std::io::Error },

    /// The source and target file paths point to the same file.
    #[error("source and target file path are the same file")]
    SourceAndTargetAreTheSameFile,

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}


/// Represents an error state when removing a file.
#[derive(Error, Debug)]
pub enum FileRemoveError {
    /// The source file cannot be found.
    #[error("file does not exist")]
    NotFound,

    /// The source file path is not a file.
    #[error("provided file path is not a file")]
    NotAFile,

    /// The file cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file")]
    UnableToAccessFile { error: std::io::Error },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}
