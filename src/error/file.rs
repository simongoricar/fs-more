use thiserror::Error;

/// Represents an error when copying or moving a file.
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

    /// The target file already exists. Some copy/move options disable this error:
    /// - [`FileCopyOptions.overwrite_existing`][crate::file::FileCopyOptions],
    /// - [`FileCopyWithProgressOptions.overwrite_existing`][crate::file::FileCopyWithProgressOptions],
    /// - [`FileMoveOptions.overwrite_existing`][crate::file::FileMoveOptions] and
    /// - [`FileMoveWithProgressOptions.overwrite_existing`][crate::file::FileMoveWithProgressOptions].
    #[error("target file already exists")]
    AlreadyExists,

    /// The target file cannot be accessed or written to (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access target file")]
    UnableToAccessTargetFile { error: std::io::Error },

    /// The source and target file paths point to the same file.
    #[error("source and target file path are the same file")]
    SourceAndTargetAreTheSameFile,

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}


/// Represents an error when removing a file.
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

/// Represents an error when querying the size of a file.
#[derive(Error, Debug)]
pub enum FileSizeError {
    /// The source file cannot be found.
    #[error("file does not exist")]
    NotFound,

    /// The source file path is not a file nor a symbolic link to a file.
    #[error("provided file path is not a file nor a symbolic link to one")]
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
