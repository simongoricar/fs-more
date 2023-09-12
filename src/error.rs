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

/// Represents an error state when querying details about a file.
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



/// Represents an error state when copying or moving a directory.
#[derive(Error, Debug)]
pub enum DirectoryError {
    /// The root source directory (the directory you want to copy) cannot be found.
    #[error("source directory path (root) does not exist")]
    SourceRootDirectoryNotFound,

    /// The provided source directory path is not a directory.
    #[error("provided source directory path is not a directory")]
    SourceRootDirectoryIsNotADirectory,

    /// A directory or file in the source directory has dissapeared since being scanned
    /// by the same function.
    #[error("a directory or file inside the source directory has dissapeared mid-copy")]
    SourceItemNotFound,

    // TODO Rework and reword this.
    /// Canonicalization or other path-related error.
    #[error("path error: {error}")]
    PathError { error: std::io::Error },

    /// A source directory or file cannot be accessed
    /// (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access source directory or file")]
    UnableToAccessSource { error: std::io::Error },

    /// A target directory or file cannot be created / written to
    /// (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access target directory or file")]
    UnableToAccessTarget { error: std::io::Error },

    /// The target directory or file already exists.
    #[error("target directory or file already exists")]
    TargetItemAlreadyExists,

    // TODO Is this used?
    /// The source and target directory paths point to the same directory.
    #[error("source and target path are the same directory")]
    SourceAndTargetAreTheSame,

    /// A scanned subdirectory's path is not inside the root directory.
    #[error("a scanned subdirectory's path is not inside the root directory")]
    SubdirectoryEscapesRoot,

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error: {error}")]
    OtherIoError { error: std::io::Error },
}

/// Represents an error state when scanning a directory
/// (see [`DirectoryScan`][crate::directory::DirectoryScan]).
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

/// Represents an error state when querying size of a scanned directory.
#[derive(Error, Debug)]
pub enum DirectorySizeScanError {
    #[error("the root directory path doesn't exist")]
    RootDirectoryNotFound,

    /// The root directory path is not a directory nor a symbolic link to a file.
    #[error(
        "provided directory path is not a directory nor a symbolic link to one"
    )]
    RootIsNotADirectory,

    /// A file that was scanned on initialization of [`DirectoryScan`][crate::directory::DirectoryScan]
    /// is no longer there or no longer a file.
    #[error("a scanned file no longer exists or isn't a file anymore")]
    FileNoLongerExists,

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
