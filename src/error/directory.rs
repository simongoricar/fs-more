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

    /// A broken symbolic link has been encountered inside the source directory.
    ///
    /// This error can occur only when `broken_symlink_behaviour` is set to
    /// [`BrokenSymlinkBehaviour::Abort`].
    ///
    ///
    /// [`BrokenSymlinkBehaviour::Abort`]: crate::directory::BrokenSymlinkBehaviour::Abort
    #[error(
        "symbolic link inside source directory is broken, \
        and the behaviour is set to abort"
    )]
    SymbolicLinkIsBroken {
        /// Path of the broken symbolic link.
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

    /// A directory copy planning error.
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
        /// The path we were unable to access.
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
        /// The file path that could not be copied.
        file_path: PathBuf,

        /// The underlying file copying error.
        #[source]
        error: FileError,
    },

    /// An error occurred while trying to create a symlink at the destination.
    #[error(
        "failed while creating a symlink at {}",
        .symlink_path.display()
    )]
    SymlinkCreationError {
        /// The path to the symbolic link that could not be created.
        symlink_path: PathBuf,

        /// The underlying symlink creation error.
        #[source]
        error: std::io::Error,
    },

    /// A destination directory, a file, or a sub-directory inside it
    /// has changed since the preparation phase of the directory copy.
    ///
    /// We can't guarantee that all destination directory changes
    /// will trigger this, but some more obvious problematic ones, like
    /// a file appearing in one of the destinations we wanted to copy to, will.
    ///
    /// This is essentially an unavoidable
    /// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// bug, but we try to catch it if possible.
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file has been created externally mid-execution: {}", .path.display())]
    DestinationEntryUnexpected {
        /// The path of the target directory or file that already exists.
        path: PathBuf,
    },
}



/// Directory copying error (see [`copy_directory`] / [`copy_directory_with_progress`]).
///
///
/// [`copy_directory`]: crate::directory::copy_directory
/// [`copy_directory_with_progress`]: crate::directory::copy_directory_with_progress
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
        /// The path we were unable to access.
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

    /// A destination directory, a file, or a sub-directory inside it
    /// has changed since the preparation phase of the directory move call.
    ///
    /// We can't guarantee that all destination directory changes
    /// will trigger this, but some more obvious problematic ones, like
    /// a file appearing in one of the destinations we wanted to copy to, will.
    ///
    /// This is essentially an unavoidable
    /// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// bug, but we try to catch it if possible.
    ///
    /// The `path` field contains the path that already existed, causing this error.
    #[error("destination directory or file has been created externally mid-execution: {}", .path.display())]
    DestinationEntryUnexpected {
        /// The path of the target directory or file that already exists.
        path: PathBuf,
    },

    /// Directory copy execution error.
    ///
    /// These errors can happen when a move-by-rename fails
    /// and a copy-and-delete is performed instead.
    #[error(transparent)]
    CopyDirectoryError(#[from] CopyDirectoryExecutionError),

    /// Occurs when renaming is the only enabled directory move strategy,
    /// but it fails.
    ///
    /// This commonly indicates that the source and destination directory are
    /// on different mount points, which would require copy-and-delete, and sometimes
    /// even following (instead of preserving) symbolic links.
    #[error(
        "only rename strategy is enabled (with no copy-and-delete \
        fallback strategy), but we were unable to rename the directory"
    )]
    RenameFailedAndNoFallbackStrategy,

    /// An uncategorized unrecoverable IO error.
    /// See `error` field for more information.
    #[error("uncategorized std::io::Error")]
    OtherIoError {
        /// IO error describing the cause of the outer error.
        #[source]
        error: std::io::Error,
    },
}



/// Directory moving error (see [`move_directory`] / [`move_directory_with_progress`]).
///
///
/// [`move_directory`]: crate::directory::move_directory
/// [`move_directory_with_progress`]: crate::directory::move_directory_with_progress
#[derive(Error, Debug)]
pub enum MoveDirectoryError {
    /// Directory move preparation error.
    #[error(transparent)]
    PreparationError(#[from] MoveDirectoryPreparationError),

    /// Directory move execution error.
    #[error(transparent)]
    ExecutionError(#[from] MoveDirectoryExecutionError),
}



/// An error that can occur when querying the size of a directory
/// (see [`directory_size_in_bytes`]).
///
///
/// [`directory_size_in_bytes`]: crate::directory::directory_size_in_bytes
#[derive(Error, Debug)]
pub enum DirectorySizeScanError {
    /// An error occurred while scanning the directory.
    #[error("failed while scanning directory: {}", .directory_path.display())]
    ScanError {
        /// The scanning error.
        #[source]
        error: DirectoryScanError,

        /// Base directory path for the scan.
        directory_path: PathBuf,
    },
}



/// An error that can occur when checking whether a directory is empty
/// (see [`is_directory_empty`]).
///
///
/// [`is_directory_empty`]: crate::directory::is_directory_empty
#[derive(Error, Debug)]
pub enum DirectoryEmptinessScanError {
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
        /// The directory path that could not be read.
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
        /// The directory path whose entries could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given file or directory could not be read.
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
        /// The drectory path that could not be read.
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
        /// The directory path whose entries could not be read.
        directory_path: PathBuf,

        /// IO error describing why the given file or directory could not be read.
        #[source]
        error: std::io::Error,
    },

    /// A symlink inside the scan tree is cyclical.
    #[error("encountered a directory symlink cycle at {}", .directory_path.display())]
    SymlinkCycleEncountered {
        /// The directory path at which the cycle loops around (i.e. where the cycle was detected).
        directory_path: PathBuf,
    },
}
