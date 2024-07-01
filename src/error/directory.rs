use std::path::PathBuf;

use thiserror::Error;

use super::FileError;
use crate::directory::DestinationDirectoryRule;


/// Source directory path validation error.
#[derive(Error, Debug)]
pub enum SourceDirectoryPathValidationError {
    /// The source directory (path to the directory you want to copy)
    /// does not exist.
    #[error(
        "source directory path does not exist: {}",
        .directory_path.display()
    )]
    NotFound {
        /// Source directory path.
        directory_path: PathBuf,
    },

    /// The source path (path to the directory you want to copy)
    /// exists, but does not point to a directory.
    #[error(
        "source path exists, but is not a directory: {}",
         .path.display()
    )]
    NotADirectory {
        /// The base source path that was supposed to be a directory.
        path: PathBuf,
    },

    /// The source directory could not be read, or its path could not be canonicalized.
    ///
    /// Among other things, this can happen due to missing read permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access source directory: {}", .directory_path.display())]
    UnableToAccess {
        /// The exact path we are unable to access.
        directory_path: PathBuf,

        /// IO error describing why the source directory could not be accessed.
        #[source]
        error: std::io::Error,
    },
}


/// Destination directory path validation error.
#[derive(Error, Debug)]
pub enum DestinationDirectoryPathValidationError {
    /// The base source path (path to the directory you want to copy)
    /// exists, but does not point to a directory.
    #[error(
        "destination path exists, but is not a directory: {}",
         .directory_path.display()
    )]
    NotADirectory {
        /// Destination directory path.
        directory_path: PathBuf,
    },

    /// The destination directory could not be read, or its path could not be canonicalized.
    ///
    /// Among other things, this can happen due to missing read permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access destination directory: {}", .directory_path.display())]
    UnableToAccess {
        /// The exact path we were unable to access.
        directory_path: PathBuf,

        /// IO error describing why the source directory could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// A destination directory or a file inside it already exists,
    /// which is against the provided [`DestinationDirectoryRule`].
    #[error(
        "destination path already exists, which is against \
        the configured destination directory rule ({:?}): {}",
        .destination_directory_rule,
        .path.display()
    )]
    AlreadyExists {
        /// Path to the file or directory that should not exist based on the provided rules.
        path: PathBuf,

        /// Destination directory rule that made the existing destination
        /// directory invalid (see [`DestinationDirectoryRule::DisallowExisting`]).
        destination_directory_rule: DestinationDirectoryRule,
    },

    /// A destination directory or a file inside it exists and is not empty,
    /// which is against the provided [`DestinationDirectoryRule`].
    #[error(
        "destination directory exists and is not empty, which is against \
        the configured destination directory rule ({:?}): {}",
        .destination_directory_rule,
        .directory_path.display(),
    )]
    NotEmpty {
        /// Path to the destination directory that should be empty based on the provided rules.
        directory_path: PathBuf,

        /// Destination directory rule that made the existing destination
        /// directory invalid (see [`DestinationDirectoryRule::AllowEmpty`]).
        destination_directory_rule: DestinationDirectoryRule,
    },

    /// The destination directory path equals or points inside the source directory,
    /// which is very problematic for copies or moves.
    #[error(
        "destination directory path equals or points inside the source directory, \
        which is invalid: {} (but source path is {})",
        .destination_directory_path.display(),
        .source_directory_path.display()
    )]
    DescendantOfSourceDirectory {
        /// Invalid destination directory path.
        destination_directory_path: PathBuf,

        /// Corresponding source directory path.
        source_directory_path: PathBuf,
    },
}


/// Directory copy or move planning error.
#[derive(Error, Debug)]
pub enum DirectoryExecutionPlanError {
    /// A source or destination directory, one of its sub-directories or a file
    /// in it (or its metadata) cannot be read.
    ///
    /// For example, this can happen due to missing read permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access path: {}", .path.display())]
    UnableToAccess {
        /// The path we were unable to access.
        path: PathBuf,

        /// IO error describing why the path could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// An item inside the source directory "escaped" outside of
    /// the base source directory.
    ///
    /// # Implementation detail
    /// This is an extremely unlikely error, because its requirement
    /// is that [`std::fs::read_dir`]'s iterator returns a directory entry
    /// outside the provided directory path.
    ///
    /// Even though this seems extremely unlikely, a `panic!` would be
    /// an extreme measure due to the many types of filesystems that exist.
    /// Instead, treat this as a truly fatal error.
    #[error(
        "a directory entry inside the source directory escaped out of it: {}",
        .path.display()
    )]
    EntryEscapesSourceDirectory {
        /// The path that "escaped" the source directory.
        path: PathBuf,
    },

    /// A destination directory or a file inside it already exists,
    /// which is against the configured [`DestinationDirectoryRule`].
    ///
    /// This can also happen when we intended to copy a file to the destination,
    /// but a directory with the same name appeared mid-copy
    /// (an unavoidable [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use) bug).
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file already exists: {}", .path.display())]
    DestinationItemAlreadyExists {
        /// Path of the target directory or file that already exists.
        path: PathBuf,
    },
}



/// An item inside the source directory "escaped" outside of
/// the base source directory.
///
/// # Implementation detail
/// This is an extremely unlikely error, because its requirement
/// is that [`fs::read_dir`]'s iterator returns a directory entry
/// outside the provided directory path.
///
/// Even though this seems extremely unlikely, a `panic!` would be
/// an extreme measure due to the many types of filesystems that exist.
/// Instead, treat this as a truly fatal error.
#[derive(Error, Debug)]
#[error(
    "a directory entry inside the source directory escaped out of it: {}",
    .path.display()
)]
pub(crate) struct SourceSubPathNotUnderBaseSourceDirectory {
    /// The path that "escaped" the source directory.
    pub(crate) path: PathBuf,
}


/// Directory copy preparation error.
#[derive(Error, Debug)]
pub enum CopyDirectoryPreparationError {
    /// A source directory validation error.
    #[error(transparent)]
    SourceDirectoryValidationError(#[from] SourceDirectoryPathValidationError),

    /// A destination directory validation error.
    #[error(transparent)]
    DestinationDirectoryValidationError(#[from] DestinationDirectoryPathValidationError),

    /// Directory copy or move planning error.
    #[error(transparent)]
    CopyPlanningError(#[from] DirectoryExecutionPlanError),
}


/// Directory copy execution error.
#[derive(Error, Debug)]
pub enum CopyDirectoryExecutionError {
    /// Failed to create a directory inside the destination folder.
    ///
    /// For example, this can happen due to missing write permissions.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to create directory: {}", .directory_path.display())]
    UnableToCreateDirectory {
        /// Directory we were unable to create.
        directory_path: PathBuf,

        /// IO error describing why the directory could not be created.
        #[source]
        error: std::io::Error,
    },

    /// A file or directory inside the destination directory could not be accessed.
    #[error("unable to access destination path: {}", .path.display())]
    UnableToAccessDestination {
        /// Path we were unable to access.
        path: PathBuf,

        /// IO error describing why the directory could not be created.
        #[source]
        error: std::io::Error,
    },

    /// An error occurred while trying to copy a file to the destination.
    #[error(
        "an error occurred while copying a file to the destination: {}",
        .file_path.display(),
    )]
    FileCopyError {
        /// File path that could not be copied.
        file_path: PathBuf,

        /// The underlying file copying error.
        #[source]
        error: FileError,
    },

    /// A destination directory, a file or a sub-directory inside it
    /// has changed since the preparation phase of the directory copy call.
    ///
    /// We can't guarantee that all destination directory changes
    /// will trigger this, but some more obvious problematic ones, like
    /// a file appearing in one of the destinations we wanted to copy to, will.
    ///
    /// This is essentially an unavoidable
    /// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// bug.
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file has been created externally mid-execution: {}", .path.display())]
    DestinationEntryUnexpected {
        /// Path of the target directory or file that already exists.
        path: PathBuf,
    },
}


/// Directory copying error, see [`copy_directory`].
///
///
/// [`copy_directory`]: [crate::directory::copy_directory]
#[derive(Error, Debug)]
pub enum CopyDirectoryError {
    /// Directory copy preparation error.
    #[error(transparent)]
    PreparationError(#[from] CopyDirectoryPreparationError),

    /// Directory copy execution error.
    #[error(transparent)]
    ExecutionError(#[from] CopyDirectoryExecutionError),
}


/// Directory move preparation error.
#[derive(Error, Debug)]
pub enum MoveDirectoryPreparationError {
    /// Source directory validation error.
    #[error(transparent)]
    SourceDirectoryValidationError(#[from] SourceDirectoryPathValidationError),

    /// Destination directory validation error.
    #[error(transparent)]
    DestinationDirectoryValidationError(#[from] DestinationDirectoryPathValidationError),

    /// Source directory entry scanning error.
    #[error(transparent)]
    DirectoryScanError(#[from] DirectoryScanError),

    /// Source directory size scanning error.
    #[error(transparent)]
    DirectorySizeScanError(#[from] DirectorySizeScanError),

    /// Directory copy planning error. These errors can happen
    /// when a move-by-rename fails and a copy-and-delete is attempted instead.
    #[error(transparent)]
    CopyPlanningError(#[from] DirectoryExecutionPlanError),
}


/// Directory move execution error.
#[derive(Error, Debug)]
pub enum MoveDirectoryExecutionError {
    /// A file or directory inside the source directory could not be accessed.
    #[error("unable to access source path: {}", .path.display())]
    UnableToAccessSource {
        /// Path we were unable to access.
        path: PathBuf,

        /// IO error describing why the directory could not be created.
        #[source]
        error: std::io::Error,
    },

    /// An item inside the source directory "escaped" outside of
    /// the base source directory.
    ///
    /// # Implementation detail
    /// This is an extremely unlikely error, because its requirement
    /// is that [`std::fs::read_dir`]'s iterator returns a directory entry
    /// outside the provided directory path.
    ///
    /// Even though this seems extremely unlikely, a `panic!` would be
    /// an extreme measure due to the many types of filesystems that exist.
    /// Instead, treat this as a truly fatal error.
    #[error(
        "a directory entry inside the source directory escaped out of it: {}",
        .path.display()
    )]
    EntryEscapesSourceDirectory {
        /// The path that "escaped" the source directory.
        path: PathBuf,
    },

    /// A destination directory, a file or a sub-directory inside it
    /// has changed since the preparation phase of the directory move call.
    ///
    /// We can't guarantee that all destination directory changes
    /// will trigger this, but some more obvious problematic ones, like
    /// a file appearing in one of the destinations we wanted to copy to, will.
    ///
    /// This is essentially an unavoidable
    /// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// bug.
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file has been created externally mid-execution: {}", .path.display())]
    DestinationEntryUnexpected {
        /// Path of the target directory or file that already exists.
        path: PathBuf,
    },

    /// Directory copy execution error. These errors can happen
    /// when a move-by-rename fails and a copy-and-delete is performed instead.
    #[error(transparent)]
    CopyDirectoryError(#[from] CopyDirectoryExecutionError),

    /// An uncategorized unrecoverable IO error. See `error` for more information.
    #[error("uncategorized std::io::Error")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        #[source]
        error: std::io::Error,
    },
}



/// Directory moving error, see [`move_directory_with_progress`].
///
/// [`move_directory_with_progress`]: [crate::directory::move_directory_with_progress]
#[derive(Error, Debug)]
pub enum MoveDirectoryError {
    /// Directory move preparation error.
    #[error(transparent)]
    PreparationError(#[from] MoveDirectoryPreparationError),

    /// Directory move execution error.
    #[error(transparent)]
    ExecutionError(#[from] MoveDirectoryExecutionError),
}



/// An error that can occur when copying or moving a directory.
#[derive(Error, Debug)]
pub enum DirectoryError {
    /// The base source directory (i.e. the directory you want to copy from) does not exist.
    #[error(
        "source directory path does not exist: {}",
        .directory_path.display()
    )]
    SourceDirectoryNotFound {
        /// Source directory path.
        directory_path: PathBuf,
    },

    /// The base source directory path (i.e. the directory you want to copy from) exists,
    /// but does not point to a directory.
    #[error(
        "source directory path exists, but is not a directory: {}",
         .directory_path.display()
    )]
    SourceDirectoryNotADirectory {
        /// Source directory path.
        directory_path: PathBuf,
    },

    /// A base source directory, its sub-directory or a file inside it cannot be read.
    ///
    /// For example, this can happen due to missing permissions,
    /// files or directories being removed externally mid-copy or mid-move, etc.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access source directory or file: {}", .path.display())]
    UnableToAccessSource {
        /// The path we are unable to access.
        path: PathBuf,

        /// IO error describing why the source directory could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// A base source directory, its sub-directory or a file inside it
    /// no longer exists (since being first scanned when preparing for a copy, move etc.).
    ///
    /// This is basically a [TOCTOU](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// race condition.
    #[error(
        "directory or file inside the source directory has been \
        unexpectedly removed while processing: {}",
        .path.display()
    )]
    SourceEntryNoLongerExists {
        /// The path to a directory or file that is invalid.
        path: PathBuf,
    },

    /// The provided destination directory path points to an invalid location.
    ///
    /// This can occur due to (not an exhaustive list):
    /// - source and destination directory are the same,
    /// - destination directory is a subdirectory of the source directory, or,
    /// - destination path already exists, but is not a directory.
    #[error("destination directory path points to an invalid location: {}", .path.display())]
    InvalidDestinationDirectoryPath {
        /// Invalid destination path.
        path: PathBuf,
    },

    /// The file system state of the destination directory does not match
    /// the provided [`DestinationDirectoryRule`].
    ///
    /// For example, this happens when the the destination directory rule is set to
    /// [`DestinationDirectoryRule::AllowEmpty`], but the destination directory isn't actually empty.
    #[error(
        "destination directory is not empty, but configured rules ({:?}) require so: {}",
        destination_directory_rule,
        .destination_path.display()
    )]
    DestinationDirectoryNotEmpty {
        /// Destination directory path.
        destination_path: PathBuf,

        /// Requirements for the destination directory
        /// (e.g. it should be empty or it should not exist at all).
        destination_directory_rule: DestinationDirectoryRule,
    },

    /// A destination directory or a file inside it cannot be created
    /// or written to (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access destination directory or file: {}", .path.display())]
    UnableToAccessDestination {
        /// Path that cannot be accessed.
        path: PathBuf,

        /// IO error describing why the target directory could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// A destination directory or a file inside it already exists.
    ///
    /// This can also happen when we intended to copy a file to the destination,
    /// but a directory with the same name appeared mid-copy
    /// (an unavoidable [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use) bug).
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file already exists: {}", .path.display())]
    DestinationItemAlreadyExists {
        /// Path of the target directory or file that already exists.
        path: PathBuf,
    },

    /// An item inside the source directory somehow escaped outside
    /// the base source directory.
    #[error(
        "a sub-path inside the source directory escaped out of it: {}",
        .path.display()
    )]
    SourceSubPathEscapesSourceDirectory {
        /// The related path that "escaped" the source directory.
        path: PathBuf,
    },

    /// An uncategorized unrecoverable IO error. See `error` for more information.
    #[error("uncategorized std::io::Error")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        #[source]
        error: std::io::Error,
    },
}



/// An error that can occur when scanning a directory.
#[derive(Error, Debug)]
pub enum DirectoryScanError {
    /// The provided directory path to scan doesn't exist.
    #[error("path doesn't exist: {}", .path.display())]
    NotFound {
        /// The directory path that couldn't be scanned.
        path: PathBuf,
    },

    /// The provided directory path exists, but is not a directory.
    #[error(
        "path exists, but is not a directory nor a symlink to one: {}",
        .path.display()
    )]
    NotADirectory {
        /// The directory path that couldn't be scanned.
        path: PathBuf,
    },

    /// The provided directory path is a directory,
    /// but could not be read due to an IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to read directory: {}", .directory_path.display())]
    UnableToReadDirectory {
        /// Directory path that could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given root directory could not be read.
        #[source]
        error: std::io::Error,
    },

    /// A directory contains an entry (i.e. directory or file)
    /// that could not be read due to an IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to read directory entry for {}", .directory_path.display())]
    UnableToReadDirectoryItem {
        /// Directory path whose entries could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given file or directory could not be read.
        #[source]
        error: std::io::Error,
    },
}



/// An error that can occur when querying size of a scanned directory.
#[derive(Error, Debug)]
pub enum DirectorySizeScanError {
    /// The provided directory path does not exist.
    #[error("the provided scan directory path doesn't exist: {}", .path.display())]
    ScanDirectoryNotFound {
        /// The directory whose scan was requested.
        path: PathBuf,
    },

    /// The root directory path exists, but is not a directory nor a symbolic link to one.
    #[error(
        "the provided scan path exists, bus is not a directory \
        nor a symbolic link to one: {}", .path.display()
    )]
    ScanDirectoryNotADirectory {
        /// The path that was requested to be scanned.
        path: PathBuf,
    },

    /// A file or directory that was scanned on initialization
    /// of [`DirectoryScan`][crate::directory::DirectoryScan] is no longer there or no longer a file.
    ///
    /// This is basically a [TOCTOU](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    ///
    #[error("a scanned file or directory no longer exists or isn't a file anymore: {path}")]
    ScanEntryNoLongerExists {
        /// Path of the file or directory that no longer exists.
        path: PathBuf,
    },

    /// A file cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access file: {}", .file_path.display())]
    UnableToAccessFile {
        /// File path that could not be accessed.
        file_path: PathBuf,

        /// Underlying IO error describing why the file could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// The directory cannot be accessed (e.g. due to missing permissions).
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to access directory: {}", .directory_path.display())]
    UnableToAccessDirectory {
        /// Directory path that could not be accessed.
        directory_path: PathBuf,

        /// Underlying IO error describing why the directory could not be accessed.
        #[source]
        error: std::io::Error,
    },

    /// Some other [`std::io::Error`] was encountered.
    #[error("other std::io::Error")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        #[source]
        error: std::io::Error,
    },
}



/// An error that can occur when checking whether a directory is empty.
#[derive(Error, Debug)]
pub enum IsDirectoryEmptyError {
    /// The provided path doesn't exist.
    #[error("given path does not exist: {}", .directory_path.display())]
    NotFound {
        /// Directory path that does not exist.
        directory_path: PathBuf,
    },

    /// The provided path exists, but is not a directory.
    #[error("given path exists, but is not a directory: {}", .path.display())]
    NotADirectory {
        /// Path that exists, but should have been a directory.
        path: PathBuf,
    },

    /// Could not read the contents of a directory.
    ///
    /// For example, this can happen due to missing permissions.
    #[error("unable to read contents of directory: {}", .directory_path.display())]
    UnableToReadDirectory {
        /// Directory path that could not be read.
        directory_path: PathBuf,

        /// Underlying IO error describing why the directory could not be read.
        #[source]
        error: std::io::Error,
    },
}


/// An error that can occur when scanning a directory.
#[derive(Error, Debug)]
pub enum DirectoryScanErrorV2 {
    /// The provided directory path to scan doesn't exist.
    #[error("path doesn't exist: {}", .path.display())]
    NotFound {
        /// The directory path that couldn't be scanned.
        path: PathBuf,
    },

    /// The provided directory path exists, but is not a directory.
    #[error(
        "path exists, but is not a directory nor a symlink to one: {}",
        .path.display()
    )]
    NotADirectory {
        /// The directory path that couldn't be scanned.
        path: PathBuf,
    },

    /// The provided directory path is a directory,
    /// but could not be read due to an IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to read directory: {}", .directory_path.display())]
    UnableToReadDirectory {
        /// Directory path that could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given root directory could not be read.
        #[source]
        error: std::io::Error,
    },

    /// A directory contains an entry (i.e. directory or file)
    /// that could not be read due to an IO error.
    ///
    /// The inner [`std::io::Error`] will likely describe a more precise cause of this error.
    #[error("unable to read directory entry for {}", .directory_path.display())]
    UnableToReadDirectoryEntry {
        /// Directory path whose entries could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given file or directory could not be read.
        #[source]
        error: std::io::Error,
    },
}
