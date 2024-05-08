use std::path::PathBuf;

use thiserror::Error;


/// An error that can occur when copying or moving a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum FileError {
    /// The provided source file path does not exist.
    #[error("source file does not exist: {}", .path.display())]
    SourceFileNotFound {
        /// The path that does not exist.
        path: PathBuf,
    },

    /// The provided source file path exists, but is not a file.
    #[error("source path exists, but is not a file: {}", .path.display())]
    SourcePathNotAFile {
        /// The path that exists, but is not a file.
        path: PathBuf,
    },

    /// The source file cannot be accessed, for example due to missing permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access source file: {}", .path.display())]
    UnableToAccessSourceFile {
        /// File path that could not be accessed.
        path: PathBuf,

        /// Underlying IO error describing why the source file could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// The source file path could not be canonicalized.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    ///
    /// See also: [`std::fs::canonicalize`].
    #[error("unable to canonicalize source file path: {}", .path.display())]
    UnableToCanonicalizeSourceFilePath {
        /// Source file path that could not be canonicalized.
        path: PathBuf,

        /// Underlying IO error describing why the source file could not be canonicalized.
        #[source]
        error: std::io::Error,
    },

    /// The destination file already exists.
    ///
    /// Certain file copy and move options can disable this error:
    /// - [`FileCopyOptions.existing_destination_file_behaviour`],
    /// - [`FileCopyWithProgressOptions.existing_destination_file_behaviour`],
    /// - [`FileMoveOptions.existing_destination_file_behaviour`], and
    /// - [`FileMoveWithProgressOptions.existing_destination_file_behaviour`].
    ///
    ///
    /// [FileCopyOptions.existing_destination_file_behaviour]: [crate::file::FileCopyOptions.existing_destination_file_behaviour]
    /// [FileCopyWithProgressOptions.existing_destination_file_behaviour]: [crate::file::FileCopyWithProgressOptions.existing_destination_file_behaviour]
    /// [FileMoveOptions.existing_destination_file_behaviour]: [crate::file:FileMoveOptions]
    /// [FileMoveWithProgressOptions.existing_destination_file_behaviour]: [crate::file:FileMoveWithProgressOptions]
    #[error("destination path already exists: {}", .path.display())]
    DestinationPathAlreadyExists {
        /// Destination file path that already exists.
        path: PathBuf,
    },

    /// The destination file cannot be accessed or written to,
    /// for example due to missing permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access destination file: {}", .path.display())]
    UnableToAccessDestinationFile {
        /// Destination file path that could not be accessed.
        path: PathBuf,

        /// Underlying IO error describing why the destination file could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// The destination file path could not be canonicalized.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    ///
    /// See also: [`std::fs::canonicalize`].
    #[error("unable to canonicalize destination file path: {}", .path.display())]
    UnableToCanonicalizeDestinationFilePath {
        /// Destination file path that could not be canonicalized.
        path: PathBuf,

        /// Underlying IO error describing why the destination file could not be canonicalized.
        #[source]
        error: std::io::Error,
    },

    /// The source and destination file paths point to the same file.
    #[error("source and destination file path are the same file: {}", .path.display())]
    SourceAndDestinationAreTheSame {
        /// The conflicting source and destination path.
        path: PathBuf,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("uncategorized std::io::Error")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        #[source]
        error: std::io::Error,
    },
}


/// An error that can occur when removing a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum FileRemoveError {
    /// The provided source file path does not exist.
    #[error("source file does not exist: {}", .path.display())]
    NotFound {
        /// The path that does not exist.
        path: PathBuf,
    },

    /// The provided source file path exists, but is not a file.
    #[error("source path exists, but is not a file: {}", .path.display())]
    NotAFile {
        /// The path that exists, but is not a file.
        path: PathBuf,
    },

    /// The file cannot be accessed, for example due to missing permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file: {}", .path.display())]
    UnableToAccessFile {
        /// Path to the file that could not be accessed.
        path: PathBuf,

        /// Underlying IO error describing why the file could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// Uncategorized IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("uncategorized IO error")]
    OtherIoError {
        /// IO error describing the cause of the error.
        #[source]
        error: std::io::Error,
    },
}

/// An error that can occur when querying the size of a file.
#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum FileSizeError {
    /// The source file does not exist.
    #[error("file does not exist: {}", .path.display())]
    NotFound {
        /// Path to the file that does not exist.
        path: PathBuf,
    },

    /// The source path exists, but is not a file nor a symbolic link to one.
    #[error("provided path exists, but is not a file nor a symbolic link to one: {}", .path.display())]
    NotAFile {
        /// Path that exists, but is not a file.
        path: PathBuf,
    },

    /// The file cannot be accessed, for example due to missing permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("unable to access file: {}", .file_path.display())]
    UnableToAccessFile {
        /// Path to the file that could not be accessed.
        file_path: PathBuf,

        /// Underlying IO error describing why the file could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// Uncategorized IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe the real cause of this error.
    #[error("uncategorized IO error")]
    OtherIoError {
        /// IO error describing the cause of the error.
        #[source]
        error: std::io::Error,
    },
}
