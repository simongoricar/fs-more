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
    UnableToAccessSource {
        /// IO error describing why the source directory could not be accessed.
        error: std::io::Error,
    },

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
    UnableToAccessTarget {
        /// IO error describing why the target directory could not be accessed.
        error: std::io::Error,
    },

    /// A target directory or file already exists.
    /// The `path` field contains the path that already existed and caused this error.
    #[error("target directory or file already exists: {}", .path.display())]
    TargetItemAlreadyExists {
        /// Path of the target directory or file that already exists.
        path: PathBuf,
    },

    /// Some other unrecoverable error with some `reason`.
    #[error("an unrecoverable error has been encountered: {reason}")]
    OtherReason {
        /// Error reason.
        reason: String,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        error: std::io::Error,
    },
}

/// An error that can occur when scanning a directory.
#[derive(Error, Debug)]
pub enum DirectoryScanError {
    /// The provided directory path doesn't exist.
    #[error("the root directory path doesn't exist")]
    NotFound,

    /// The provided directory path exists, but is not a directory.
    #[error("the root directory path doesn't lead to a directory")]
    NotADirectory,

    /// The provided directory path is a directory,
    /// but could not be read due to an IO error.
    #[error("unable to read directory: {error}")]
    UnableToReadDirectory {
        /// IO error describing why the given root directory could not be read.
        error: std::io::Error,
    },

    /// The provided directory path is a directory,
    /// but contains an item (i.e. directory or file)
    /// that could not be read due to an IO error.
    #[error("unable to read directory item: {error}")]
    UnableToReadDirectoryItem {
        /// IO error describing why the given file or directory could not be read.
        error: std::io::Error,
    },
}

/// An error that can occur when querying size of a scanned directory.
#[derive(Error, Debug)]
pub enum DirectorySizeScanError {
    /// The provided directory path does not exist.
    #[error("the root directory path doesn't exist")]
    RootDirectoryNotFound,

    /// The root directory path exists, but is not a directory nor a symbolic link to one.
    #[error("provided directory path is not a directory nor a symbolic link to one")]
    RootIsNotADirectory,

    /// A file or directory that was scanned on initialization
    /// of [`DirectoryScan`][crate::directory::DirectoryScan]
    /// is no longer there or no longer a file.
    #[error("scanned file or directory no longer exists or isn't a file anymore: {path}")]
    EntryNoLongerExists {
        /// Path of the file or directory that no longer exists.
        path: PathBuf,
    },

    /// The file cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file")]
    UnableToAccessFile {
        /// Underlying IO error describing why the file could not be accessed.
        error: std::io::Error,
    },

    /// The directory cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file")]
    UnableToAccessDirectory {
        /// Underlying IO error describing why the directory could not be accessed.
        error: std::io::Error,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        error: std::io::Error,
    },
}

/// An error that can occur when checking whether a directory is empty.
#[derive(Error, Debug)]
pub enum IsDirectoryEmptyError {
    /// The provided path doesn't exist.
    #[error("given path does not exist")]
    NotFound,

    /// The provided path exists, but is not a directory.
    #[error("given path does not lead to a directory")]
    NotADirectory,

    /// Could not read the contents of some directory.
    #[error("unable to read contents of directory due to an IO error: {error}")]
    UnableToReadDirectory {
        /// Underlying IO error describing why the directory could not be read.
        error: std::io::Error,
    },
}
