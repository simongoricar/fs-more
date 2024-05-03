use std::path::Path;

use_enabled_fs_module!();

use super::{
    copy::copy_file_with_progress_unchecked,
    validate_source_file_path,
    CopyFileWithProgressOptions,
    ExistingFileBehaviour,
    FileProgress,
};
use crate::{
    error::{FileError, FileRemoveError},
    file::ValidatedSourceFilePath,
    use_enabled_fs_module,
};


/// Options that influence the [`move_file`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MoveFileOptions {
    /// How to behave for destination files that already exist.
    pub existing_destination_file_behaviour: ExistingFileBehaviour,
}

#[allow(clippy::derivable_impls)]
impl Default for MoveFileOptions {
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
pub enum MoveFileFinished {
    /// Destination file was freshly created and the contents of the source
    /// file were moved. `method` will describe how the move was made.
    Created {
        /// The number of bytes transferred in the move (i.e. the file size).
        bytes_copied: u64,

        /// How the move was accomplished.
        method: MoveFileMethod,
    },

    /// Destination file existed, but was overwritten with the contents of
    /// the source file.
    Overwritten {
        /// The number of bytes transferred in the move (i.e. the file size).
        bytes_copied: u64,

        /// How the move was accomplished.
        method: MoveFileMethod,
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
pub enum MoveFileMethod {
    /// The source file was renamed to the destination file.
    ///
    /// This is highly performant on most file systems.
    Rename,

    /// The source file was copied to the destination,
    /// and the source file was deleted afterwards.
    ///
    /// This is used if [`Self::Rename`] is impossible, and is
    /// as fast as writes normally are.
    CopyAndDelete,
}


/// Moves a single file from the source to the destination path.
///
/// The destination path must be a *file* path, and must not point to a directory.
///
///
/// ## Return value
/// The function returns [`MoveFileFinished`], which indicates whether the file was created,
/// overwritten or skipped, and includes the number of bytes moved, if relevant.
///
///
/// ## Options
/// See [`CopyFileOptions`].
///
///
/// ## Symbolic links
/// If `source_file_path` leads to a symbolic link to a file,
/// the contents of the link destination file will be copied to `destination_file_path`.
/// Finally, the original `source_file_path` symbolic link will be removed.
///
/// In short, the link target will be untouched, but the link itself will
/// not be preserved; the destination will be a normal file - a copy of the source.
///
///
/// ## Internals
/// This function will first attempt to move the file with [`std::fs::rename`].
/// If that fails (e.g. if paths are on different mount points / drives), a copy-and-delete will be attempted.
///
///
/// [`CopyFileOptions`]: [super::CopyFileOptions]
pub fn move_file<S, D>(
    source_file_path: S,
    destination_file_path: D,
    options: MoveFileOptions,
) -> Result<MoveFileFinished, FileError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file,
    } = validate_source_file_path(source_file_path)?;


    // Ensure the destination file path doesn't exist yet
    // (unless `options.existing_destination_file_behaviour` allows it),
    // and that it isn't a directory.

    let destination_file_exists = match destination_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path =
                    validated_source_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToAccessSourceFile {
                            path: validated_source_file_path.clone(),
                            error,
                        }
                    })?;

                let canonicalized_target_path =
                    destination_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToAccessDestinationFile {
                            path: destination_file_path.to_path_buf(),
                            error,
                        }
                    })?;


                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndDestinationAreTheSame {
                        path: canonicalized_source_path,
                    });
                }
            }


            if exists {
                match options.existing_destination_file_behaviour {
                    ExistingFileBehaviour::Abort => {
                        return Err(FileError::DestinationPathAlreadyExists {
                            path: destination_file_path.to_path_buf(),
                        });
                    }
                    ExistingFileBehaviour::Skip => {
                        return Ok(MoveFileFinished::Skipped);
                    }
                    ExistingFileBehaviour::Overwrite => {}
                }
            }

            exists
        }
        Err(error) => {
            return Err(FileError::UnableToAccessDestinationFile {
                path: destination_file_path.to_path_buf(),
                error,
            })
        }
    };

    // All checks have passed. Now we do the following:
    // - if both paths reside on the same filesystem
    //   (as indicated by std::fs::rename succeeding) that's nice (and fast),
    // - otherwise we need to copy to target and remove source.

    // Note that we *must not* go for the renaming shortcut if the user-provided path was actually a symbolic link to a file.
    // In that case, we need to copy the file behind the symbolic link, then remove the symbolic link.
    if !original_was_symlink_to_file
        && fs::rename(&validated_source_file_path, destination_file_path).is_ok()
    {
        // Get size of file that we just renamed.
        let target_file_path_metadata = fs::metadata(destination_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        match destination_file_exists {
            true => Ok(MoveFileFinished::Overwritten {
                bytes_copied: target_file_path_metadata.len(),
                method: MoveFileMethod::Rename,
            }),
            false => Ok(MoveFileFinished::Created {
                bytes_copied: target_file_path_metadata.len(),
                method: MoveFileMethod::Rename,
            }),
        }
    } else {
        // Copy to destination, then delete original file.
        // Special case: if the original was a symlink to a file, we need to
        // delete the symlink, not the file it points to.

        let num_bytes_copied = fs::copy(&validated_source_file_path, destination_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        let file_path_to_remove = if original_was_symlink_to_file {
            source_file_path
        } else {
            validated_source_file_path.as_path()
        };

        super::remove_file(file_path_to_remove).map_err(|error| match error {
            FileRemoveError::NotFound { path } => FileError::SourceFileNotFound { path },
            FileRemoveError::NotAFile { path } => FileError::SourcePathNotAFile { path },
            FileRemoveError::UnableToAccessFile { path, error } => {
                FileError::UnableToAccessSourceFile { path, error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;


        match destination_file_exists {
            true => Ok(MoveFileFinished::Overwritten {
                bytes_copied: num_bytes_copied,
                method: MoveFileMethod::CopyAndDelete,
            }),
            false => Ok(MoveFileFinished::Created {
                bytes_copied: num_bytes_copied,
                method: MoveFileMethod::CopyAndDelete,
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
    /// Defaults to 64 KiB.
    ///
    ///
    /// [`copy_file_with_progress`]: [super::copy_file_with_progress]
    pub progress_update_byte_interval: u64,
}

impl Default for FileMoveWithProgressOptions {
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            // 64 KiB
            read_buffer_size: 1024 * 64,
            // 64 KiB
            write_buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Moves a single file from the `source_file_path` to the `target_file_path` with progress reporting.
///
/// The destination path must be a *file* path, and must not point to a directory.
///
/// ## Return value
/// The function returns [`MoveFileFinished`], which indicates whether the file was created,
/// overwritten or skipped, and includes the number of bytes moved, if relevant.
///
///
/// ## Progress handling
/// You must also provide a progress handler that receives a
/// [`&FileProgress`][super::FileProgress] on each progress update.
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`][FileMoveWithProgressOptions::progress_update_byte_interval] option.
/// That option is the *minumum* amount of bytes written between two progress reports, meaning we can't guarantee
/// a specific amount of progress reports per file size.
/// We do, however, guarantee at least one progress report (the final one).
///
///
/// ## Options
/// See [`FileMoveWithProgressOptions`] for more details.
///
///
/// ## Symbolic links
/// If the `source_file_path` is a symbolic link to a file, the contents of the file that the link points to
/// will be copied to the `target_file_path` and the original `source_file_path` symbolic link will be removed
/// (i.e. the link destination will be untouched, but we won't preserve the link on the destination file).
///
///
/// ## Internals
/// This function will first attempt to move the file with [`std::fs::rename`].
/// If that fails (e.g. if paths are on different mount points / drives), a copy-and-delete will be attempted.
pub fn move_file_with_progress<S, D, P>(
    source_file_path: S,
    destination_file_path: D,
    options: FileMoveWithProgressOptions,
    mut progress_handler: P,
) -> Result<MoveFileFinished, FileError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    P: FnMut(&FileProgress),
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file,
    } = validate_source_file_path(source_file_path)?;


    // Ensure the destination file path doesn't exist yet
    // (unless `options.existing_destination_file_behaviour` allows it),
    // and that it isn't a directory.

    let destination_file_exists = match destination_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path =
                    validated_source_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToAccessSourceFile {
                            path: validated_source_file_path.clone(),
                            error,
                        }
                    })?;
                let canonicalized_target_path =
                    destination_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToAccessDestinationFile {
                            path: destination_file_path.to_path_buf(),
                            error,
                        }
                    })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndDestinationAreTheSame {
                        path: canonicalized_source_path,
                    });
                }
            }


            if exists {
                match options.existing_destination_file_behaviour {
                    ExistingFileBehaviour::Abort => {
                        return Err(FileError::DestinationPathAlreadyExists {
                            path: destination_file_path.to_path_buf(),
                        });
                    }
                    ExistingFileBehaviour::Skip => {
                        return Ok(MoveFileFinished::Skipped);
                    }
                    ExistingFileBehaviour::Overwrite => {}
                }
            }

            exists
        }
        Err(error) => {
            return Err(FileError::UnableToAccessDestinationFile {
                path: destination_file_path.to_path_buf(),
                error,
            })
        }
    };

    // All checks have passed. Now we do the following:
    // - if both paths reside on the same filesystem
    //   (as indicated by std::fs::rename succeeding)
    //   that's nice and fast (we mustn't forget to do at least one progress report),
    // - otherwise we need to copy to target and remove source.

    if !original_was_symlink_to_file
        && fs::rename(&validated_source_file_path, destination_file_path).is_ok()
    {
        // Get size of file that we just renamed, emit one progress report, and return.

        let target_file_path_size_bytes = fs::metadata(destination_file_path)
            .map_err(|error| FileError::OtherIoError { error })?
            .len();

        progress_handler(&FileProgress {
            bytes_finished: target_file_path_size_bytes,
            bytes_total: target_file_path_size_bytes,
        });


        match destination_file_exists {
            true => Ok(MoveFileFinished::Overwritten {
                bytes_copied: target_file_path_size_bytes,
                method: MoveFileMethod::Rename,
            }),
            false => Ok(MoveFileFinished::Created {
                bytes_copied: target_file_path_size_bytes,
                method: MoveFileMethod::Rename,
            }),
        }
    } else {
        // It's impossible for us to just rename the file,
        // so we need to copy and delete the original.

        let bytes_written = copy_file_with_progress_unchecked(
            &validated_source_file_path,
            destination_file_path,
            CopyFileWithProgressOptions {
                existing_destination_file_behaviour: options.existing_destination_file_behaviour,
                read_buffer_size: options.read_buffer_size,
                write_buffer_size: options.write_buffer_size,
                progress_update_byte_interval: options.progress_update_byte_interval,
            },
            progress_handler,
        )?;


        let file_path_to_remove = if original_was_symlink_to_file {
            source_file_path
        } else {
            validated_source_file_path.as_path()
        };

        super::remove_file(file_path_to_remove).map_err(|error| match error {
            FileRemoveError::NotFound { path } => FileError::SourceFileNotFound { path },
            FileRemoveError::NotAFile { path } => FileError::SourcePathNotAFile { path },
            FileRemoveError::UnableToAccessFile { path, error } => {
                FileError::UnableToAccessSourceFile { path, error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;


        match destination_file_exists {
            true => Ok(MoveFileFinished::Overwritten {
                bytes_copied: bytes_written,
                method: MoveFileMethod::CopyAndDelete,
            }),
            false => Ok(MoveFileFinished::Created {
                bytes_copied: bytes_written,
                method: MoveFileMethod::CopyAndDelete,
            }),
        }
    }
}
