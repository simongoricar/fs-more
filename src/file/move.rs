#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::Path;

#[cfg(feature = "fs-err")]
use fs_err as fs;

use super::{
    copy::copy_file_with_progress_unchecked,
    validate_source_file_path,
    FileCopyWithProgressOptions,
    FileProgress,
};
use crate::{
    error::{FileError, FileRemoveError},
    file::ValidatedSourceFilePath,
};

/// Options that influence the [`move_file`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileMoveOptions {
    /// Whether to allow overwriting the target file if it already exists.
    pub overwrite_existing: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FileMoveOptions {
    fn default() -> Self {
        Self {
            overwrite_existing: false,
        }
    }
}


/// Moves a single file from the `source_file_path` to the `target_file_path`.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
/// ## Options
/// If `options.overwrite_existing` is `true`, an existing target file will be overwritten.
///
/// If `options.overwrite_existing` is `false` and the target file exists, this function will
/// return `Err` with [`FileError::AlreadyExists`][crate::error::FileError::AlreadyExists].
///
/// ## Symbolic links
/// If the `source_file_path` is a symbolic link to a file, the contents of the file that the link points to
/// will be copied to the `target_file_path` and the original `source_file_path` symbolic link will be removed
/// (i.e. the link destination will be untouched, but we won't preserve the link on the target file).
///
/// ## Internals
/// This function will first attempt to move the file with [`std::fs::rename`].
/// If that fails (you can't rename files across filesystems), a copy-and-delete will be performed.
pub fn move_file<P, T>(
    source_file_path: P,
    target_file_path: T,
    options: FileMoveOptions,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let target_file_path = target_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file,
    } = validate_source_file_path(source_file_path)?;

    // Ensure the target file path doesn't exist yet
    // (unless `overwrite_existing` is `true`)
    // and that it isn't already a directory path.
    match target_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path = validated_source_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessSourceFile { error })?;
                let canonicalized_target_path = target_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessTargetFile { error })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndTargetAreTheSameFile);
                }
            }

            if exists && !options.overwrite_existing {
                return Err(FileError::AlreadyExists);
            }
        }
        Err(error) => return Err(FileError::UnableToAccessTargetFile { error }),
    }

    // All checks have passed. Now we do the following:
    // - if both paths reside on the same filesystem
    //   (as indicated by std::fs::rename succeeding) that's nice (and fast),
    // - otherwise we need to copy to target and remove source.

    // Note that we *must not* go for the renaming shortcut if the user-provided path was actually a symbolic link to a file.
    // In that case, we need to copy the file behind the symbolic link, then remove the symbolic link.
    if !original_was_symlink_to_file
        && fs::rename(&validated_source_file_path, target_file_path).is_ok()
    {
        // Get size of file that we just renamed.
        let target_file_path_metadata = target_file_path
            .metadata()
            .map_err(|error| FileError::OtherIoError { error })?;

        Ok(target_file_path_metadata.len())
    } else {
        // Copy, then delete original.
        let num_bytes_copied = fs::copy(&validated_source_file_path, target_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        let file_path_to_remove = if original_was_symlink_to_file {
            source_file_path
        } else {
            validated_source_file_path.as_path()
        };

        super::remove_file(file_path_to_remove).map_err(|error| match error {
            FileRemoveError::NotFound => FileError::NotFound,
            FileRemoveError::NotAFile => FileError::NotAFile,
            FileRemoveError::UnableToAccessFile { error } => {
                FileError::UnableToAccessSourceFile { error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;

        Ok(num_bytes_copied)
    }
}


/// Options that influence the [`move_file_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileMoveWithProgressOptions {
    /// Whether to allow overwriting the target file if it already exists.
    pub overwrite_existing: bool,

    /// Internal buffer size (for both reading and writing) when copying the file,
    /// defaults to 64 KiB.
    pub buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
    ///
    /// *Note that the interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for FileMoveWithProgressOptions {
    fn default() -> Self {
        Self {
            overwrite_existing: false,
            // 64 KiB
            buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Moves a single file from the `source_file_path` to the `target_file_path`.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
/// You must also provide a progress handler that receives a
/// [`&FileProgress`][super::FileProgress] on each progress update.
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`][FileMoveWithProgressOptions::progress_update_byte_interval] option.
/// That option is the *minumum* amount of bytes written between two progress reports, meaning we can't guarantee
/// a specific amount of progress reports per file size.
/// We do, however, guarantee at least one progress report (the final one).
///
/// ## Options
/// If [`options.overwrite_existing`][FileMoveWithProgressOptions::overwrite_existing] is `true`,
/// an existing target file will be overwritten.
///
/// If [`options.overwrite_existing`][FileMoveWithProgressOptions::overwrite_existing] is `false`
/// and the target file exists, this function will return `Err`
/// with [`FileError::AlreadyExists`][crate::error::FileError::AlreadyExists].
///
/// ## Symbolic links
/// If the `source_file_path` is a symbolic link to a file, the contents of the file that the link points to
/// will be copied to the `target_file_path` and the original `source_file_path` symbolic link will be removed
/// (i.e. the link destination will be untouched, but we won't preserve the link on the target file).
///
/// ## Internals
/// This function will first attempt to move the file with [`std::fs::rename`].
/// If that fails (you can't rename files across filesystems), a copy-and-delete will be performed.
pub fn move_file_with_progress<P, T, F>(
    source_file_path: P,
    target_file_path: T,
    options: FileMoveWithProgressOptions,
    mut progress_handler: F,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&FileProgress),
{
    let source_file_path = source_file_path.as_ref();
    let target_file_path = target_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path: validated_source_file_path,
        original_was_symlink_to_file,
    } = validate_source_file_path(source_file_path)?;

    // Ensure the target file path doesn't exist yet
    // (unless `overwrite_existing` is `true`)
    // and that it isn't already a directory path.
    match target_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path = validated_source_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessSourceFile { error })?;
                let canonicalized_target_path = target_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessTargetFile { error })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndTargetAreTheSameFile);
                }
            }

            if exists && !options.overwrite_existing {
                return Err(FileError::AlreadyExists);
            }
        }
        Err(error) => return Err(FileError::UnableToAccessTargetFile { error }),
    }

    // All checks have passed. Now we do the following:
    // - if both paths reside on the same filesystem
    //   (as indicated by std::fs::rename succeeding)
    //   that's nice and fast (we mustn't forget to do at least one progress report),
    // - otherwise we need to copy to target and remove source.

    if !original_was_symlink_to_file
        && fs::rename(&validated_source_file_path, target_file_path).is_ok()
    {
        // Get size of file that we just renamed.
        let target_file_path_size_bytes = target_file_path
            .metadata()
            .map_err(|error| FileError::OtherIoError { error })?
            .len();

        progress_handler(&FileProgress {
            bytes_finished: target_file_path_size_bytes,
            bytes_total: target_file_path_size_bytes,
        });

        Ok(target_file_path_size_bytes)
    } else {
        // It's impossible to rename the file, so we need to copy and delete the original.
        let bytes_written = copy_file_with_progress_unchecked(
            &validated_source_file_path,
            target_file_path,
            FileCopyWithProgressOptions {
                overwrite_existing: options.overwrite_existing,
                skip_existing: false,
                buffer_size: options.buffer_size,
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
            FileRemoveError::NotFound => FileError::NotFound,
            FileRemoveError::NotAFile => FileError::NotAFile,
            FileRemoveError::UnableToAccessFile { error } => {
                FileError::UnableToAccessSourceFile { error }
            }
            FileRemoveError::OtherIoError { error } => FileError::OtherIoError { error },
        })?;

        Ok(bytes_written)
    }
}
