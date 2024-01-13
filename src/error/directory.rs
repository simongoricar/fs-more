use std::path::PathBuf;

use thiserror::Error;

/// An error that can occur when copying or moving a directory.
#[derive(Error, Debug)]
pub enum DirectoryError {
    /// The root source directory (the directory you want to copy from) cannot be found.
    #[error("provided source directory path does not exist")]
    SourceDirectoryNotFound,

    /// The provided source directory path is not a directory.
    #[error("provided source directory path is not a directory")]
    SourceDirectoryIsNotADirectory,

    /// A source directory or file cannot be read.
    /// This can happen, among other things, due to missing permissions or files/directories being removed externally mid-copy or mid-move.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access source directory or file")]
    UnableToAccessSource { error: std::io::Error },

    /// A directory or file in the source directory
    /// has disappeared since being scanned by the same function.
    #[error("a directory or file inside the source directory has been removed mid-process")]
    SourceContentsInvalid,

    /// The target directory path points to an invalid location, because (one of):
    /// - source and target directory are the same,
    /// - target directory is a subdirectory of the source directory, or,
    /// - target path already exists and is not a directory.
    #[error("target directory path points to an invalid location")]
    InvalidTargetDirectoryPath,

    /// Returned when the the target directory rule is set to
    /// [`TargetDirectoryRule::AllowEmpty`][crate::directory::TargetDirectoryRule::AllowEmpty],
    /// but the given target directory isn't empty.
    #[error("target directory is not empty, but configured rules require so")]
    TargetDirectoryIsNotEmpty,

    /// A target directory or file cannot be created / written to
    /// (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access target directory or file")]
    UnableToAccessTarget { error: std::io::Error },

    /// A target directory or file already exists.
    /// The `path` field contains the path that already existed and caused this error.
    #[error("target directory or file already exists: {}", .path.display())]
    TargetItemAlreadyExists { path: PathBuf },

    /// Some other unrecoverable error with some `reason`.
    #[error("an unrecoverable error has been encountered: {reason}")]
    OtherReason { reason: String },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}

/// An error that can occur when scanning a directory.
#[derive(Error, Debug)]
pub enum DirectoryScanError {
    #[error("the root directory path doesn't exist")]
    NotFound,

    #[error("the root directory path doesn't lead to a directory")]
    NotADirectory,

    #[error("unable to read directory: {error}")]
    UnableToReadDirectory { error: std::io::Error },

    #[error("unable to read directory item: {error}")]
    UnableToReadDirectoryItem { error: std::io::Error },
}

/// An error that can occur when querying size of a scanned directory.
#[derive(Error, Debug)]
pub enum DirectorySizeScanError {
    #[error("the root directory path doesn't exist")]
    RootDirectoryNotFound,

    /// The root directory path is not a directory nor a symbolic link to a file.
    #[error("provided directory path is not a directory nor a symbolic link to one")]
    RootIsNotADirectory,

    /// A file or directory that was scanned on initialization
    /// of [`DirectoryScan`][crate::directory::DirectoryScan]
    /// is no longer there or no longer a file.
    #[error("scanned file or directory no longer exists or isn't a file anymore: {path}")]
    EntryNoLongerExists { path: PathBuf },

    /// The file cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file")]
    UnableToAccessFile { error: std::io::Error },

    /// The directory cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file")]
    UnableToAccessDirectory { error: std::io::Error },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}

/// An error that can occur when checking whether a directory is empty.
#[derive(Error, Debug)]
pub enum IsDirectoryEmptyError {
    #[error("given path does not exist")]
    NotFound,

    #[error("given path does not lead to a directory")]
    NotADirectory,

    #[error("unable to read contents of directory due to an std::io::Error: {error}")]
    UnableToReadDirectory { error: std::io::Error },
}
