use std::path::Path;

use_enabled_fs_module!();

use super::{
    copy::copy_file_with_progress_unchecked,
    validate_destination_file_path,
    validate_source_file_path,
    DestinationValidationAction,
    ExistingFileBehaviour,
    FileCopyWithProgressOptions,
    FileProgress,
};
use crate::{
    error::{FileError, FileRemoveError},
    file::ValidatedSourceFilePath,
    DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
    DEFAULT_READ_BUFFER_SIZE,
    DEFAULT_WRITE_BUFFER_SIZE,
};


/// Options that influence the [`move_file`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileMoveOptions {
    /// How to behave for destination files that already exist.
    pub existing_destination_file_behaviour: ExistingFileBehaviour,
}

#[allow(clippy::derivable_impls)]
impl Default for FileMoveOptions {
    /// Constructs a default [`FileMoveOptions`]:
    /// - existing destination files will not be overwritten, and will cause an error ([`ExistingFileBehaviour::Abort`]).
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        }
    }
}



/// Information about a successful file move operation.
///
/// See also: [`move_file`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileMoveFinished {
    /// Destination file was freshly created and the contents of the source
    /// file were moved. `method` will describe how the move was made.
    Created {
        /// The number of bytes transferred in the move (i.e. the file size).
        bytes_copied: u64,

        /// How the move was accomplished.
        method: FileMoveMethod,
    },

    /// Destination file existed, and was overwritten with the contents of
    /// the source file.
    Overwritten {
        /// The number of bytes transferred in the move (i.e. the file size).
        bytes_copied: u64,

        /// How the move was accomplished.
        method: FileMoveMethod,
    },

    /// File was not moved because the destination file already existed.
    ///
    /// This can be returned by [`move_file`] or [`move_file_with_progress`]
    /// if `options.existing_destination_file_behaviour` is set to [`ExistingFileBehaviour::Skip`].
    ///
    /// Note that this means the source file still exists.
    Skipped,
}


/// A method used for moving a file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileMoveMethod {
    /// The source file was renamed to the destination file.
    ///
    /// This is very highly performant on most file systems,
    /// to the point of being near instantaneous.
    Rename,

    /// The source file was copied to the destination,
    /// and the source file was deleted afterwards.
    ///
    /// This is generally used only if [`Self::Rename`] is impossible,
    /// and is as fast as writes normally are.
    CopyAndDelete,
}


/// Moves a single file from the source to the destination path.
///
/// The destination path must be a *file* path, and must not point to a directory.
///
///
/// # Symbolic links
/// Symbolic links are generally preserved, unless renaming them fails.
///
/// This means the following: if `source_file_path` leads to a symbolic link that points to a file,
/// we'll try to move the file by renaming it to the destination path, even if it is a symbolic link to a file.
/// If that fails, the contents of the file the symlink points to will instead
/// be *copied*, then the symlink at `source_file_path` itself will be removed.
///
/// This matches `mv` behaviour on Unix[^unix-mv].
///
///
/// # Options
/// See [`FileMoveOptions`] for available file moving options.
///
///
/// # Return value
/// If the move succeeds, the function returns [`FileMoveFinished`],
/// which indicates whether the file was created,
/// overwritten or skipped. The struct also includes the number of bytes moved,
/// if relevant.
///
///
/// # Errors
/// If the file cannot be moved to the destination, a [`FileError`] is returned;
/// see its documentation for more details. Here is a non-exhaustive list of error causes:
/// - If the source path has issues (does not exist, does not have the correct permissions, etc.),
///   one of [`SourceFileNotFound`], [`SourcePathNotAFile`] or [`UnableToAccessSourceFile`]
///   variants will be returned.
/// - If the destination already exists, and [`options.existing_destination_file_behaviour`]
///   is set to [`ExistingFileBehaviour::Abort`], then a [`DestinationPathAlreadyExists`]
///   will be returned.
/// - If the source and destination paths are canonically actually the same file,
///   then copying will be aborted with [`SourceAndDestinationAreTheSame`].
/// - If the destination path has other issues (is a directory, does not have the correct permissions, etc.),
///   [`UnableToAccessDestinationFile`] will be returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// <br>
///
/// #### See also
/// If you are looking for a file moving function that reports progress,
/// see [`move_file_with_progress`].
///
///
/// <br>
///
/// <details>
/// <summary><h4>Implementation details</h4></summary>
///
/// *This section describes internal implementations details.
/// They should not be relied on, because they are informative
/// and may change in the future.*
///
/// <br>
///
/// This function will first attempt to move the file by renaming it using [`std::fs::rename`]
/// (or [`fs_err::rename`](https://docs.rs/fs-err/latest/fs_err/fn.rename.html) if
/// the `fs-err` feature flag is enabled).
///
/// If the rename fails, for example when the source and destination path are on different
/// mount points or drives, a copy-and-delete will be performed instead.
///
/// The method used will be reflected in the [`FileMoveMethod`] used in the return value.
///
/// </details>
///
///
/// [`options.existing_destination_file_behaviour`]: FileMoveOptions::existing_destination_file_behaviour
/// [`SourceFileNotFound`]: FileError::SourceFileNotFound
/// [`SourcePathNotAFile`]: FileError::SourcePathNotAFile
/// [`UnableToAccessSourceFile`]: FileError::UnableToAccessSourceFile
/// [`DestinationPathAlreadyExists`]: FileError::DestinationPathAlreadyExists
/// [`UnableToAccessDestinationFile`]: FileError::UnableToAccessDestinationFile
/// [`SourceAndDestinationAreTheSame`]: FileError::SourceAndDestinationAreTheSame
/// [`OtherIoError`]: FileError::OtherIoError
/// [^unix-mv]: Source for coreutils' `mv` is available
///   [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/mv.c#L196-L244).
pub fn move_file<S, D>(
    source_file_path: S,
    destination_file_path: D,
    options: FileMoveOptions,
) -> Result<FileMoveFinished, FileError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();


    let validated_source_path = validate_source_file_path(source_file_path)?;

    let (validated_destination_file_path, destination_file_exists) =
        match validate_destination_file_path(
            &validated_source_path,
            destination_file_path,
            options.existing_destination_file_behaviour,
        )? {
            DestinationValidationAction::SkipCopyOrMove => {
                return Ok(FileMoveFinished::Skipped);
            }
            DestinationValidationAction::Continue(info) => {
                (info.destination_file_path, info.exists)
            }
        };

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file: source_file_was_symlink_to_file,
    } = validated_source_path;


    // All checks have passed. Now we do the following:
    // - Try to move by renaming the source file. If that succeeds,
    //   that's nice and fast (and symlink-preserving).
    // - Otherwise, we need to copy the source (or the file underneath it,
    //   if it a symlink) to target and remove the source.


    let source_file_path_to_rename = if source_file_was_symlink_to_file {
        source_file_path
    } else {
        validated_source_file_path.as_path()
    };

    if fs::rename(source_file_path_to_rename, &validated_destination_file_path).is_ok() {
        // Get size of file that we just renamed.
        let target_file_path_metadata = fs::metadata(&validated_destination_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        match destination_file_exists {
            true => Ok(FileMoveFinished::Overwritten {
                bytes_copied: target_file_path_metadata.len(),
                method: FileMoveMethod::Rename,
            }),
            false => Ok(FileMoveFinished::Created {
                bytes_copied: target_file_path_metadata.len(),
                method: FileMoveMethod::Rename,
            }),
        }
    } else {
        // Copy to destination, then delete original file.
        // Special case: if the original was a symlink to a file, we need to
        // delete the symlink, not the file it points to.

        let num_bytes_copied =
            fs::copy(&validated_source_file_path, validated_destination_file_path)
                .map_err(|error| FileError::OtherIoError { error })?;

        let source_file_path_to_remove = if source_file_was_symlink_to_file {
            // `source_file_path` instead of `validated_source_file_path` is intentional:
            // if the source was a symlink, we should remove the link, not its destination.
            source_file_path
        } else {
            validated_source_file_path.as_path()
        };

        super::remove_file(source_file_path_to_remove).map_err(|error| match error {
            FileRemoveError::NotFound { path } => FileError::SourceFileNotFound { path },
            FileRemoveError::NotAFile { path } => FileError::SourcePathNotAFile { path },
            FileRemoveError::UnableToAccessFile { path, error } => {
                FileError::UnableToAccessSourceFile { path, error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;


        match destination_file_exists {
            true => Ok(FileMoveFinished::Overwritten {
                bytes_copied: num_bytes_copied,
                method: FileMoveMethod::CopyAndDelete,
            }),
            false => Ok(FileMoveFinished::Created {
                bytes_copied: num_bytes_copied,
                method: FileMoveMethod::CopyAndDelete,
            }),
        }
    }
}



/// Options that influence the [`move_file_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileMoveWithProgressOptions {
    /// How to behave for destination files that already exist.
    pub existing_destination_file_behaviour: ExistingFileBehaviour,

    /// Internal buffer size used for reading the source file.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to the destination file.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// The smallest amount of bytes processed between two consecutive progress reports.
    ///
    /// Increase this value to make progress reports less frequent, and decrease it
    /// to make them more frequent.
    ///
    /// *Note that this is the minimum;* the real reporting interval can be larger.
    /// Consult [`copy_file_with_progress`] documentation for more details.
    ///
    /// Defaults to 512 KiB.
    ///
    ///
    /// [`copy_file_with_progress`]: [super::copy_file_with_progress]
    pub progress_update_byte_interval: u64,
}

impl Default for FileMoveWithProgressOptions {
    /// Constructs a default [`FileMoveOptions`]:
    /// - existing destination files will not be overwritten, and will cause an error ([`ExistingFileBehaviour::Abort`]),
    /// - read and write buffers with be 64 KiB large,
    /// - the progress report closure interval will be 512 KiB.
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            write_buffer_size: DEFAULT_WRITE_BUFFER_SIZE,
            progress_update_byte_interval: DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
        }
    }
}


/// Moves a single file from the source to the destination path, with progress reporting
///
/// The destination path must be a *file* path, and must not point to a directory.
///
///
/// # Symbolic links
/// Symbolic links are generally preserved, unless renaming them fails.
///
/// This means the following: if `source_file_path` leads to a symbolic link that points to a file,
/// we'll try to move the file by renaming it to the destination path, even if it is a symbolic link to a file.
/// If that fails, the contents of the file the symlink points to will instead
/// be *copied*, then the symlink at `source_file_path` itself will be removed.
///
/// This matches `mv` behaviour on Unix[^unix-mv].
///
///
/// # Options
/// See [`FileMoveWithProgressOptions`] for available file moving options.
///
///
/// # Return value
/// If the move succeeds, the function returns [`FileMoveFinished`],
/// which indicates whether the file was created,
/// overwritten or skipped. The struct also includes the number of bytes moved,
/// if relevant.
///
///
/// ## Progress handling
/// This function allows you to receive progress reports by passing
/// a `progress_handler` closure. It will be called with
/// a reference to [`FileProgress`] regularly.
///
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`] option.
/// The value of this option is the minimum amount of bytes written to a file between
/// two calls to the provided `progress_handler`.
///
/// This function does not guarantee a precise amount of progress reports per file size
/// and progress reporting interval setting; it does, however, guarantee at least one progress report:
/// the final one, which happens when the file has been completely copied.
/// In most cases though, the number of calls to
/// the closure will be near the expected amount,
/// which is `file_size / progress_update_byte_interval`.
///
///
/// # Errors
/// If the file cannot be moved to the destination, a [`FileError`] is returned;
/// see its documentation for more details. Here is a non-exhaustive list of error causes:
/// - If the source path has issues (does not exist, does not have the correct permissions, etc.),
///   one of [`SourceFileNotFound`], [`SourcePathNotAFile`], or [`UnableToAccessSourceFile`],
///   variants will be returned.
/// - If the destination already exists, and [`options.existing_destination_file_behaviour`]
///   is set to [`ExistingFileBehaviour::Abort`], then a [`DestinationPathAlreadyExists`]
///   will be returned.
/// - If the source and destination paths are canonically actually the same file,
///   then copying will be aborted with [`SourceAndDestinationAreTheSame`].
/// - If the destination path has other issues (is a directory, does not have the correct permissions, etc.),
///   [`UnableToAccessDestinationFile`] will be returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// <br>
///
/// #### See also
/// If you are looking for a file moving function that does not report progress,
/// see [`move_file`].
///
///
/// <br>
///
/// <details>
/// <summary><h4>Implementation details</h4></summary>
///
/// *This section describes internal implementations details.
/// They should not be relied on, because they are informative
/// and may change in the future.*
///
/// <br>
///
/// This function will first attempt to move the file by renaming it using [`std::fs::rename`]
/// (or [`fs_err::rename`](https://docs.rs/fs-err/latest/fs_err/fn.rename.html) if
/// the `fs-err` feature flag is enabled).
///
/// If the rename fails, for example when the source and destination path are on different
/// mount points or drives, a copy-and-delete will be performed instead.
///
/// The method used will be reflected in the [`FileMoveMethod`] used in the return value.
///
/// </details>
///
///
/// [`options.progress_update_byte_interval`]: FileMoveWithProgressOptions::progress_update_byte_interval
/// [`options.existing_destination_file_behaviour`]: FileMoveWithProgressOptions::existing_destination_file_behaviour
/// [`SourceFileNotFound`]: FileError::SourceFileNotFound
/// [`SourcePathNotAFile`]: FileError::SourcePathNotAFile
/// [`UnableToAccessSourceFile`]: FileError::UnableToAccessSourceFile
/// [`DestinationPathAlreadyExists`]: FileError::DestinationPathAlreadyExists
/// [`UnableToAccessDestinationFile`]: FileError::UnableToAccessDestinationFile
/// [`SourceAndDestinationAreTheSame`]: FileError::SourceAndDestinationAreTheSame
/// [`OtherIoError`]: FileError::OtherIoError
/// [^unix-mv]: Source for coreutils' `mv` is available
///   [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/mv.c#L196-L244).
pub fn move_file_with_progress<S, D, P>(
    source_file_path: S,
    destination_file_path: D,
    options: FileMoveWithProgressOptions,
    mut progress_handler: P,
) -> Result<FileMoveFinished, FileError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    P: FnMut(&FileProgress),
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();


    let validated_source_path = validate_source_file_path(source_file_path)?;

    let (validated_destination_file_path, destination_file_exists) =
        match validate_destination_file_path(
            &validated_source_path,
            destination_file_path,
            options.existing_destination_file_behaviour,
        )? {
            DestinationValidationAction::SkipCopyOrMove => {
                return Ok(FileMoveFinished::Skipped);
            }
            DestinationValidationAction::Continue(info) => {
                (info.destination_file_path, info.exists)
            }
        };

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file: source_file_was_symlink_to_file,
    } = validated_source_path;


    // All checks have passed. Now we do the following:
    // - Try to move by renaming the source file. If that succeeds,
    //   that's nice and fast (and symlink-preserving). We must also not forget
    //   to do one progress report.
    // - Otherwise, we need to copy the source (or the file underneath it,
    //   if it a symlink) to target and remove the source.

    let source_file_path_to_rename = if source_file_was_symlink_to_file {
        source_file_path
    } else {
        validated_source_file_path.as_path()
    };

    if fs::rename(source_file_path_to_rename, &validated_destination_file_path).is_ok() {
        // Get size of file that we just renamed, emit one progress report, and return.

        let target_file_path_size_bytes = fs::metadata(&validated_destination_file_path)
            .map_err(|error| FileError::OtherIoError { error })?
            .len();

        progress_handler(&FileProgress {
            bytes_finished: target_file_path_size_bytes,
            bytes_total: target_file_path_size_bytes,
        });


        match destination_file_exists {
            true => Ok(FileMoveFinished::Overwritten {
                bytes_copied: target_file_path_size_bytes,
                method: FileMoveMethod::Rename,
            }),
            false => Ok(FileMoveFinished::Created {
                bytes_copied: target_file_path_size_bytes,
                method: FileMoveMethod::Rename,
            }),
        }
    } else {
        // It's impossible for us to just rename the file,
        // so we need to copy and delete the original.

        let bytes_written = copy_file_with_progress_unchecked(
            &validated_source_file_path,
            &validated_destination_file_path,
            FileCopyWithProgressOptions {
                existing_destination_file_behaviour: options.existing_destination_file_behaviour,
                read_buffer_size: options.read_buffer_size,
                write_buffer_size: options.write_buffer_size,
                progress_update_byte_interval: options.progress_update_byte_interval,
            },
            progress_handler,
        )?;


        let source_file_path_to_remove = if source_file_was_symlink_to_file {
            // `source_file_path` instead of `validated_source_file_path` is intentional:
            // if the source was a symlink, we should remove the link, not its destination.
            source_file_path
        } else {
            validated_source_file_path.as_path()
        };

        super::remove_file(source_file_path_to_remove).map_err(|error| match error {
            FileRemoveError::NotFound { path } => FileError::SourceFileNotFound { path },
            FileRemoveError::NotAFile { path } => FileError::SourcePathNotAFile { path },
            FileRemoveError::UnableToAccessFile { path, error } => {
                FileError::UnableToAccessSourceFile { path, error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;


        match destination_file_exists {
            true => Ok(FileMoveFinished::Overwritten {
                bytes_copied: bytes_written,
                method: FileMoveMethod::CopyAndDelete,
            }),
            false => Ok(FileMoveFinished::Created {
                bytes_copied: bytes_written,
                method: FileMoveMethod::CopyAndDelete,
            }),
        }
    }
}
