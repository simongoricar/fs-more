use thiserror::Error;

/// An error that can occur when copying or moving a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
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
    UnableToAccessSourceFile {
        /// Underlying IO error describing why the source file could not be accessed.
        error: std::io::Error,
    },

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
    UnableToAccessTargetFile {
        /// Underlying IO error describing why the target file could not be accessed.
        error: std::io::Error,
    },

    /// The source and target file paths point to the same file.
    #[error("source and target file path are the same file")]
    SourceAndTargetAreTheSameFile,

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        error: std::io::Error,
    },
}


/// An error that can occur when removing a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
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
    UnableToAccessFile {
        /// Underlying IO error describing why the file could not be accessed.
        error: std::io::Error,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        error: std::io::Error,
    },
}

/// An error that can occur when querying the size of a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
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
    UnableToAccessFile {
        /// Underlying IO error describing why the file could not be accessed.
        error: std::io::Error,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        error: std::io::Error,
    },
}
