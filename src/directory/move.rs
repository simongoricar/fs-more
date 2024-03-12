#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "fs-err")]
use fs_err as fs;

use super::{
    copy_directory_unchecked,
    perform_prepared_copy_directory_with_progress,
    prepared::{
        validate_source_directory_path,
        validate_source_target_directory_pair,
        validate_target_directory_path,
        PreparedDirectoryCopy,
        ValidatedTargetPath,
    },
    DirectoryCopyOperation,
    DirectoryCopyOptions,
    DirectoryCopyWithProgressOptions,
    DirectoryScan,
    TargetDirectoryRule,
};
use crate::{
    error::{DirectoryError, DirectoryScanError, DirectorySizeScanError},
    file::FileProgress,
};

/// Options that influence the [`move_directory`] function.
pub struct DirectoryMoveOptions {
    /// Specifies whether you allow the target directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// target files or subdirectories to already exist
    /// (and whether you allow them to be overwritten).
    ///
    /// See [`TargetDirectoryRule`] for more details and examples.
    pub target_directory_rule: TargetDirectoryRule,
}

impl Default for DirectoryMoveOptions {
    fn default() -> Self {
        Self {
            target_directory_rule: TargetDirectoryRule::AllowEmpty,
        }
    }
}


/// Describes strategies for moving a directory.
///
/// Included in [`FinishedDirectoryMove`] to allow callers
/// to understand how the directory was moved.
///
/// To clarify: *the caller can not request that a specific move strategy be used*.
/// This enum is simply included in the return value to help the caller understand how the move was performed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectoryMoveStrategy {
    /// The source directory was simply renamed from the source path to the target path.
    ///
    /// **This is the fastest method**, but generally works only if both paths are
    /// on the same mount point / drive.
    RenameSourceDirectory,

    /// The *contents* of the source directory were renamed from their source paths
    /// to their target pahts. This is Windows-specific behaviour since it doesn't allow
    /// renaming to existing (even if empty) target directories.
    ///
    /// *This should be almost as fast as [`Self::RenameSourceDirectory`]*,
    /// but just like it, this method generally works only if both paths are
    /// on the same mount point / drive.
    RenameSourceDirectoryContents,

    /// The source directory was recursively copied to the target directory,
    /// with the source directory being deleted afterwards.
    ///
    /// Out of the three methods given, this is the slowest. It is also unavoidable
    /// if the directory can't renamed, which can happen when the source and target
    /// directory exist on different mount points or drives.
    CopyAndDelete,
}

/// Describes actions taken by the [`move_directory`] function.
///
/// This is the return value of [`move_directory`] and [`move_directory_with_progress`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FinishedDirectoryMove {
    /// Total amount of bytes moved.
    pub total_bytes_moved: u64,

    /// Number of files moved (created).
    pub num_files_moved: usize,

    /// Number of directories moved (created).
    pub num_directories_moved: usize,

    /// How the directory was moved: was is simply renamed or was it copied and deleted.
    pub used_strategy: DirectoryMoveStrategy,
}

/// Summarizes the contents of a directory for internal use.
struct DirectoryContentDetails {
    /// Total size of the directory in bytes.
    pub(crate) total_bytes: u64,

    /// Total number of files in the directory (recursive).
    pub(crate) total_files: usize,

    /// Total number of subdirectories in the directory (recursive).
    pub(crate) total_directories: usize,
}

/// Scans the provided directory (no depth limit).
/// 
/// The returned struct contains information about the total number files and directories 
/// and the total directory size.
#[rustfmt::skip]
fn collect_source_directory_details(
    source_directory_path: &Path
) -> Result<DirectoryContentDetails, DirectoryError> {
    let scan = DirectoryScan::scan_with_options(source_directory_path, None, false)
        .map_err(|error| match error {
            DirectoryScanError::NotFound => 
                DirectoryError::SourceDirectoryNotFound,
            DirectoryScanError::NotADirectory => 
                DirectoryError::SourceDirectoryIsNotADirectory,
            DirectoryScanError::UnableToReadDirectory { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectoryScanError::UnableToReadDirectoryItem { error } => 
                DirectoryError::UnableToAccessSource { error },
        })?;

    let total_size_in_bytes = scan.total_size_in_bytes()
        .map_err(|error| match error {
            DirectorySizeScanError::RootDirectoryNotFound => 
                DirectoryError::SourceDirectoryNotFound,
            DirectorySizeScanError::RootIsNotADirectory => 
                DirectoryError::SourceDirectoryIsNotADirectory,
            DirectorySizeScanError::EntryNoLongerExists { path } => 
                DirectoryError::UnableToAccessSource {
                    error: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Scaned entry has been removed before it could be processed: {}",
                            path.display()
                        ),
                    ),
                },
            DirectorySizeScanError::UnableToAccessFile { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectorySizeScanError::UnableToAccessDirectory { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectorySizeScanError::OtherIoError { error } => 
                DirectoryError::OtherIoError { error },
        })?;

    Ok(DirectoryContentDetails {
        total_bytes: total_size_in_bytes,
        total_files: scan.files.len(),
        total_directories: scan.directories.len(),
    })
}


/// Attempts a directory move by using the [`std::fs::rename`]
/// (or `fs_err::rename` is using the `fs-err` feature).
///
/// - If a trivial rename is performed, `Ok(Some(...))` is returned.
/// - If a trivial rename is not possible (e.g. target directory is not empty) or fails,
///   the function returns `Ok(None)`. This can happend when source and target are on different mount points.
///   This return value indicates you need to perform a copy-and-delete instead.
/// - If an error occurs, the function returns `Err(DirectoryError)`.
fn attempt_directory_move_by_rename(
    source_directory_path: &Path,
    source_directory_details: &DirectoryContentDetails,
    validated_target: &ValidatedTargetPath,
) -> Result<Option<FinishedDirectoryMove>, DirectoryError> {
    // We can attempt to simply rename the directory. This is much faster,
    // but will fail if the source and target paths aren't on the same mount point or filesystem
    // or, if on Windows, the target directory already exists.

    if !validated_target.is_empty_directory.unwrap_or(true) {
        // Indicates that we can't rename (target directory is not empty).
        return Ok(None);
    }


    #[cfg(unix)]
    {
        // If the target directory exists, but is empty, we can (on Unix only)
        // directly rename the source directory to the target (this might still fail due to different mount points).
        if fs::rename(
            &source_directory_path,
            &validated_target.directory_path,
        )
        .is_ok()
        {
            return Ok(Some(FinishedDirectoryMove {
                total_bytes_moved: source_directory_details.total_bytes,
                num_files_moved: source_directory_details.total_files,
                num_directories_moved: source_directory_details.total_directories,
                used_strategy: DirectoryMoveStrategy::RenameSourceDirectory,
            }));
        }

        Ok(None)
    }

    #[cfg(windows)]
    {
        use super::rejoin_source_subpath_onto_target;

        // On Windows, `rename`'s target directory must not exist.
        if !validated_target.exists
            && fs::rename(
                source_directory_path,
                &validated_target.directory_path,
            )
            .is_ok()
        {
            return Ok(Some(FinishedDirectoryMove {
                total_bytes_moved: source_directory_details.total_bytes,
                num_files_moved: source_directory_details.total_files,
                num_directories_moved: source_directory_details.total_directories,
                used_strategy: DirectoryMoveStrategy::RenameSourceDirectory,
            }));
        }

        // Otherwise, we can rename the *contents* of the directory instead.
        // Note that this is not a recursive scan, we're simply moving (by renaming)
        // the files and directories directly inside the source directory into the target directory.
        let source_directory_contents = fs::read_dir(source_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

        for (entry_index, source_entry) in source_directory_contents.into_iter().enumerate() {
            let source_entry =
                source_entry.map_err(|error| DirectoryError::UnableToAccessSource { error })?;

            let source_path = source_entry.path();
            let target_path = rejoin_source_subpath_onto_target(
                source_directory_path,
                &source_path,
                &validated_target.directory_path,
            )?;

            let rename_result = fs::rename(source_path, target_path);

            // If we try to rename the first entry and it fails, there is a high likelihood
            // that it is due to source and target paths being on different mount points / drives
            // and that we should retry with a copy-and-delete.
            // But if the first rename succeeds and others don't, that's a clear signal
            // something else went wrong.

            if entry_index == 0 {
                if rename_result.is_err() {
                    return Ok(None);
                }
            } else {
                rename_result.map_err(|error| DirectoryError::OtherIoError { error })?;
            }
        }

        // Finally, we need to remove the (now empty) source directory path.
        fs::remove_dir(source_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessSource { error })?;


        Ok(Some(FinishedDirectoryMove {
            total_bytes_moved: source_directory_details.total_bytes,
            num_files_moved: source_directory_details.total_files,
            num_directories_moved: source_directory_details.total_directories,
            used_strategy: DirectoryMoveStrategy::RenameSourceDirectoryContents,
        }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        compile_error!(
            "fs-more supports only the following values of target_family: \
            unix and windows (notably, wasm is unsupported)."
        );
    }
}

/// Move a directory from `source_directory_path` to `target_directory_path`.
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `target_directory_path` will be created.
///
///
/// ### Warnings
/// *This function does not follow or move symbolic links in the source directory.*
///
/// It does, however, support `source_directory_path` itself being a symbolic link to a directory
/// (it will not copy the symbolic link itself, but the contents of the link destination).
///
///
/// ### Target directory
/// Depending on the [`options.target_directory_rule`][DirectoryMoveOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - with [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see [variant fields][TargetDirectoryRule::AllowNonEmpty]).
///
/// If the specified target directory rule is not satisfied,
/// a [`DirectoryError`] containing the reason will be returned before any move is performed.
///
///
/// ### Move strategies
/// Depending on the situation, the move can be performed one of two ways:
/// - The source directory can be simply renamed to the target directory.
///   This is the fastest method, but in addition to some platform-specifics<sup>*</sup> requires that the target directory is empty.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* On Windows: if the target directory already exists – even if it is empty – this function will instead
/// attempt to rename the <i>contents</i> of the source directory, see [`std::fs::rename`].
/// This will be reflected in the returned [`DirectoryMoveStrategy`], but should otherwise not be externally visible.</sup>
///
///
/// ### Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved and how the move was performed
/// (see [`FinishedDirectoryMove`]).
pub fn move_directory<S, T>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryMoveOptions,
) -> Result<FinishedDirectoryMove, DirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let source_directory_path = validate_source_directory_path(source_directory_path.as_ref())?;
    let validated_target_path = validate_target_directory_path(
        target_directory_path.as_ref(),
        &options.target_directory_rule,
    )?;

    validate_source_target_directory_pair(
        &source_directory_path,
        &validated_target_path.directory_path,
    )?;

    let source_details = collect_source_directory_details(&source_directory_path)?;


    let directory_rename_attempt = attempt_directory_move_by_rename(
        &source_directory_path,
        &source_details,
        &validated_target_path,
    )?;

    if let Some(rename_result) = directory_rename_attempt {
        return Ok(rename_result);
    }


    // At this point a simple rename was either impossible or failed.
    // We need to copy and delete instead.

    let prepared_copy = PreparedDirectoryCopy::prepare_with_validated(
        source_directory_path.clone(),
        validated_target_path,
        None,
        &options.target_directory_rule,
    )?;

    copy_directory_unchecked(
        prepared_copy,
        DirectoryCopyOptions {
            target_directory_rule: options.target_directory_rule,
            maximum_copy_depth: None,
        },
    )?;

    fs::remove_dir_all(source_directory_path)
        .map_err(|error| DirectoryError::OtherIoError { error })?;


    Ok(FinishedDirectoryMove {
        total_bytes_moved: source_details.total_bytes,
        num_files_moved: source_details.total_files,
        num_directories_moved: source_details.total_directories,
        used_strategy: DirectoryMoveStrategy::CopyAndDelete,
    })
}


/// Options that influence the [`move_directory_with_progress`] function.
pub struct DirectoryMoveWithProgressOptions {
    /// Specifies whether you allow the target directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// target files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`TargetDirectoryRule`] for more details and examples.
    pub target_directory_rule: TargetDirectoryRule,

    /// Internal buffer size (for both reading and writing) when copying files,
    /// defaults to 64 KiB.
    pub buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
    ///
    /// *Note that the real reporting interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for DirectoryMoveWithProgressOptions {
    fn default() -> Self {
        Self {
            target_directory_rule: TargetDirectoryRule::AllowEmpty,
            // 64 KiB
            buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Describes a directory move operation.
///
/// Used in progress reporting in [`move_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DirectoryMoveOperation {
    /// Describes a directory creation operation.
    CreatingDirectory {
        /// Path of the directory that is being created.
        target_path: PathBuf,
    },

    /// Describes a file being copied.
    /// For more precise copying progress, see the `progress` field.
    CopyingFile {
        /// Path of the file is being created.
        target_path: PathBuf,

        /// Progress of the file operation.
        progress: FileProgress,
    },

    /// Describes removal of the source directory
    /// (happens at the very end of moving a directory).
    RemovingSourceDirectory,
}


/// Represents the progress of moving a directory.
///
/// Used to report directory moving progress to a user-provided closure,
/// see [`move_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryMoveProgress {
    /// Amount of bytes that need to be moved for the directory move to be complete.
    pub bytes_total: u64,

    /// Amount of bytes that have been moved so far.
    pub bytes_finished: u64,

    /// Number of files that have been moved so far.
    ///
    /// If the copy-and-delete strategy is used under the hood,
    /// this can instead mean how many files have been *copied* so far
    /// (deletion will come at the end). For more information, see [`DirectoryMoveStrategy`].
    pub files_moved: usize,

    /// Number of directories that have been created so far.
    pub directories_created: usize,

    /// The current operation being performed.
    pub current_operation: DirectoryMoveOperation,

    /// The index of the current operation (starts at `0`, goes to `total_operations - 1`).
    pub current_operation_index: isize,

    /// The total amount of operations that need to be performed to move the requested directory.
    ///
    /// A single operation is one of (see [`DirectoryMoveProgress`]):
    /// - copying a file,
    /// - creating a directory or
    /// - removing the source directory (at the very end).
    pub total_operations: isize,
}


/// Moves a directory from `source_directory_path` to `target_directory_path`
/// (with progress reporting).
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `target_directory_path` will be created.
///
///
/// ### Warnings
/// *This function does not follow or move symbolic links in the source directory.*
///
/// It does, however, support `source_directory_path` itself being a symbolic link to a directory
/// (it will not copy the symbolic link itself, but the contents of the link destination).
///
///
/// ### Target directory
/// Depending on the [`options.target_directory_rule`][DirectoryMoveOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - with [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see [variant fields][TargetDirectoryRule::AllowNonEmpty]).
///
/// If the specified target directory rule is not satisfied,
/// a [`DirectoryError`] containing the reason will be returned before any move is performed.
///
///
/// ### Move strategies
/// Depending on the situation, the move will be performed one of two ways:
/// - The source directory can be simply renamed to the target directory.
///   *This is the fastest method,* but in addition to some platform-specifics<sup>*</sup> requires that the target directory is empty.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* On Windows: if the target directory already exists – even if it is empty – this function will instead
/// attempt to rename the <i>contents</i> of the source directory, see [`std::fs::rename`].
/// This will be reflected in the returned [`DirectoryMoveStrategy`], but should otherwise not be externally visible.</sup>
///
///
/// ### Progress reporting
/// Using the `progress_handler` you can provide a progress handler closure that
/// will receive a [`&DirectoryMoveProgress`][DirectoryMoveProgress] containing
/// the progress of the move.
///
/// As moving a directory can involve two distinct strategies (see above),
/// this method can only guarantee *a single progress report*:
/// - When using the [`DirectoryMoveStrategy::RenameSourceDirectory`] or [`DirectoryMoveStrategy::RenameSourceDirectoryContents`],
///   there will only be one progress report — the final one that reports a move being complete.
/// - When using the [`DirectoryMoveStrategy::CopyAndDelete`] strategy, the progress reporting will be the same
///   as described in the [`copy_directory_with_progress`][super::copy_directory_with_progress] function,
///   which is much more frequent.
///   
/// If the [`DirectoryMoveStrategy::CopyAndDelete`] strategy is used, the
/// [`options.progress_update_byte_interval`][DirectoryMoveWithProgressOptions::progress_update_byte_interval]
/// option controls the progress update frequency.
/// The value of that option is the *minimum* amount of bytes written to a single file
/// between two progress reports (defaults to 64 KiB).
/// As such, this function does not guarantee a fixed amount of progress reports per file size.
///
///
/// ### Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved and how the move was performed
/// (see [`FinishedDirectoryMove`]).
pub fn move_directory_with_progress<S, T, F>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryMoveWithProgressOptions,
    mut progress_handler: F,
) -> Result<FinishedDirectoryMove, DirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryMoveProgress),
{
    let source_directory_path = validate_source_directory_path(source_directory_path.as_ref())?;
    let validated_target_path = validate_target_directory_path(
        target_directory_path.as_ref(),
        &options.target_directory_rule,
    )?;

    validate_source_target_directory_pair(
        &source_directory_path,
        &validated_target_path.directory_path,
    )?;

    let source_details = collect_source_directory_details(&source_directory_path)?;


    // We'll first attempt to move the directory by renaming it.
    // If we don't succeed (e.g. source and target paths are on different drives),
    // we'll copy and delete instead.

    let directory_rename_attempt = attempt_directory_move_by_rename(
        &source_directory_path,
        &source_details,
        &validated_target_path,
    )?;

    if let Some(rename_result) = directory_rename_attempt {
        let final_progress_report = DirectoryMoveProgress {
            bytes_total: source_details.total_bytes,
            bytes_finished: source_details.total_bytes,
            files_moved: source_details.total_files,
            directories_created: source_details.total_directories,
            // Clarification: this is in the past tense, but in reality `attempt_directory_move_by_rename`
            // has already removed the empty source directory if needed.
            // Point is, all operations have finished at this point.
            current_operation: DirectoryMoveOperation::RemovingSourceDirectory,
            current_operation_index: 1,
            total_operations: 2,
        };

        progress_handler(&final_progress_report);

        return Ok(rename_result);
    }


    // Trivial directory rename failed or was impossible - we should copy to target
    // and delete old directory at the end.

    let copy_options = DirectoryCopyWithProgressOptions {
        target_directory_rule: options.target_directory_rule,
        buffer_size: options.buffer_size,
        progress_update_byte_interval: options.progress_update_byte_interval,
        maximum_copy_depth: None,
    };

    let prepared_copy = PreparedDirectoryCopy::prepare_with_validated(
        source_directory_path.clone(),
        validated_target_path,
        copy_options.maximum_copy_depth,
        &copy_options.target_directory_rule,
    )?;

    let directory_copy_result =
        perform_prepared_copy_directory_with_progress(prepared_copy, copy_options, |progress| {
            let move_operation = match progress.current_operation.clone() {
                DirectoryCopyOperation::CreatingDirectory { target_path } => {
                    DirectoryMoveOperation::CreatingDirectory { target_path }
                }
                DirectoryCopyOperation::CopyingFile {
                    target_path,
                    progress,
                } => DirectoryMoveOperation::CopyingFile {
                    target_path,
                    progress,
                },
            };

            // TODO It should be possible to further optimize constructing this differently
            //      so we don't need to do it every time (we only send a reference anyway).
            let move_progress = DirectoryMoveProgress {
                bytes_total: progress.bytes_total,
                bytes_finished: progress.bytes_finished,
                current_operation: move_operation,
                current_operation_index: progress.current_operation_index,
                total_operations: progress.total_operations,
                files_moved: progress.files_copied,
                directories_created: progress.directories_created,
            };

            progress_handler(&move_progress)
        })?;


    // Having fully copied the directory to the target, we now
    // remove the original (source) directory.
    fs::remove_dir_all(&source_directory_path)
        .map_err(|error| DirectoryError::OtherIoError { error })?;


    Ok(FinishedDirectoryMove {
        num_directories_moved: directory_copy_result.num_directories_created,
        total_bytes_moved: directory_copy_result.total_bytes_copied,
        num_files_moved: directory_copy_result.num_files_copied,
        used_strategy: DirectoryMoveStrategy::CopyAndDelete,
    })
}
