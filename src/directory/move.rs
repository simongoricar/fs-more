use std::path::{Path, PathBuf};

use_enabled_fs_module!();

use super::{
    copy_directory_unchecked,
    execute_prepared_copy_directory_with_progress_unchecked,
    prepared::{
        validate_destination_directory_path,
        validate_source_destination_directory_pair,
        validate_source_directory_path,
        DestinationDirectoryState,
        DirectoryCopyPrepared,
        ValidatedDestinationDirectory,
        ValidatedSourceDirectory,
    },
    CopyDirectoryDepthLimit,
    CopyDirectoryOperation,
    CopyDirectoryOptions,
    CopyDirectoryWithProgressOptions,
    DestinationDirectoryRule,
    DirectoryScan,
    DirectoryScanDepthLimit,
};
use crate::{
    error::{MoveDirectoryError, MoveDirectoryExecutionError, MoveDirectoryPreparationError},
    file::FileProgress,
    use_enabled_fs_module,
};



/// Options that influence the [`move_directory`] function.
pub struct MoveDirectoryOptions {
    /// Specifies whether you allow the target directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist
    /// (and whether you allow them to be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,
}

impl Default for MoveDirectoryOptions {
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
        }
    }
}



/// Describes a strategy for performing a directory move.
///
/// Included in [`MoveDirectoryFinished`] to allow callers
/// to understand how the directory was moved.
///
/// Note: *the caller can not request that a specific move strategy be used*.
/// This enum is simply included in the return value to help the caller understand how the move was performed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectoryMoveStrategy {
    /// The source directory was simply renamed from the source path to the target path.
    ///
    /// **This is the fastest method**, to the point of being near instantenous,
    /// but generally works only if both paths are on the same mount point or drive.
    Rename,

    /// The source directory was recursively copied to the target directory,
    /// and the source directory was deleted afterwards.
    ///
    /// Out of the three methods given, this is the slowest -- it is as fast as a normal recursive copy.
    /// It is also unavoidable if the directory can't renamed, which can happen when the source and destination
    /// directory exist on different mount points or drives.
    CopyAndDelete,
}



/// Describes actions taken by the [`move_directory`] function.
///
/// This is the return value of [`move_directory`] and [`move_directory_with_progress`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MoveDirectoryFinished {
    /// Total number of bytes moved.
    pub total_bytes_moved: u64,

    /// Number of files moved (created).
    pub files_moved: usize,

    /// Number of directories moved (created).
    pub directories_moved: usize,

    /// How the directory was moved: was is simply renamed or was it copied and deleted.
    pub strategy_used: DirectoryMoveStrategy,
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



/// Scans the provided directory for auxiliary details (without a depth limit).
/// This includes information like the total number of bytes it contains.
fn collect_source_directory_details(
    source_directory_path: &Path,
) -> Result<DirectoryContentDetails, MoveDirectoryPreparationError> {
    let scan = DirectoryScan::scan_with_options(
        source_directory_path,
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .map_err(MoveDirectoryPreparationError::DirectoryScanError)?;

    let total_size_in_bytes = scan
        .total_size_in_bytes()
        .map_err(MoveDirectoryPreparationError::DirectorySizeScanError)?;

    Ok(DirectoryContentDetails {
        total_bytes: total_size_in_bytes,
        total_files: scan.files().len(),
        total_directories: scan.directories().len(),
    })
}



pub(crate) enum DirectoryMoveByRenameAction {
    Renamed {
        finished_move: MoveDirectoryFinished,
    },
    Impossible,
}


/// Attempts a directory move by using the [`std::fs::rename`]
/// (or `fs_err::rename` is using the `fs-err` feature).
///
/// Returns [`DirectoryMoveByRenameAction`], which indicates whether the move-by-rename
/// succeeded, or failed due to source and destination being on different mount points or drives.
fn attempt_directory_move_by_rename(
    validated_source_directory: &ValidatedSourceDirectory,
    source_directory_details: &DirectoryContentDetails,
    validated_destination_directory: &ValidatedDestinationDirectory,
) -> Result<DirectoryMoveByRenameAction, MoveDirectoryExecutionError> {
    // We can attempt to simply rename the directory. This is much faster,
    // but will fail if the source and target paths aren't on the same mount point or filesystem
    // or, if on Windows, the target directory already exists.

    // If the destination directory exists and is not empty, a move by rename is not possible.
    if validated_destination_directory.state != DestinationDirectoryState::IsEmpty {
        return Ok(DirectoryMoveByRenameAction::Impossible);
    }


    #[cfg(unix)]
    {
        // If the target directory exists, but is empty, we can (on Unix only)
        // directly rename the source directory to the target (this might still fail due to different mount points).
        if fs::rename(
            &validated_source_directory.directory_path,
            &validated_destination_directory.directory_path,
        )
        .is_ok()
        {
            return Ok(DirectoryMoveByRenameAction::Renamed {
                finished_move: MoveDirectoryFinished {
                    total_bytes_moved: source_directory_details.total_bytes,
                    files_moved: source_directory_details.total_files,
                    directories_moved: source_directory_details.total_directories,
                    strategy_used: DirectoryMoveStrategy::Rename,
                },
            });
        }

        Ok(DirectoryMoveByRenameAction::Impossible)
    }

    #[cfg(windows)]
    {
        // On Windows, the destination directory in call to `rename` must not exist for it to work.
        if !validated_destination_directory.state.exists()
            && fs::rename(
                &validated_source_directory.directory_path,
                &validated_destination_directory.directory_path,
            )
            .is_ok()
        {
            return Ok(DirectoryMoveByRenameAction::Renamed {
                finished_move: MoveDirectoryFinished {
                    total_bytes_moved: source_directory_details.total_bytes,
                    files_moved: source_directory_details.total_files,
                    directories_moved: source_directory_details.total_directories,
                    strategy_used: DirectoryMoveStrategy::Rename,
                },
            });
        }

        Ok(DirectoryMoveByRenameAction::Impossible)
    }

    #[cfg(not(any(unix, windows)))]
    {
        compile_error!(
            "fs-more supports only the following values of target_family: unix and windows.\
            WASM is unsupported."
        );
    }
}



/// Moves a directory from the source to the destination directory.
///
///
/// # Symbolic links
/// If `source_directory_path` is itself a symlink to a directory,
/// we'll try to move the link itself by renaming it to the destination.
/// If the rename fails, the link will be followed and not preserved
/// by performing a directory copy, after which the symlink will be removed.
///
/// For symlinks *inside* the source directory, the behaviour is different depending on the move strategy:
/// - If a move by rename suceeds, any symbolic links inside the source directory, valid or not, will be preserved.
/// - If the copy-and-delete fallback is used, all symbolic links are followed and not preserved
///   (see details in [`copy_directory`]).
///
/// TODO what happens when the destination directory is not empty? is that even valid?
///
///
/// # Options
/// See [`MoveDirectoryOptions`] for a full set of available directory moving options.
///
///
/// # Move strategies
/// Depending on the situation, the move can be performed one of two ways:
/// - The source directory can be simply renamed to the destination directory.
///   This is the preferred (and fastest) method, and will preserve
///   the `source_directory_path` symlink, if it is one.
///   In addition to some other platform-specifics<sup>*</sup>,
///   this strategy requires that the destination directory is empty.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* Windows: the destination directory must not exist; if it does,
/// *even if it is empty*, the rename strategy will fail.</sup>
///
///
/// ### Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved and how the move was performed
/// (see [`MoveDirectoryFinished`]).
///
///
/// [`copy_directory`]: super::copy_directory
pub fn move_directory<S, T>(
    source_directory_path: S,
    destination_directory_path: T,
    options: MoveDirectoryOptions,
) -> Result<MoveDirectoryFinished, MoveDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let validated_source_directory = validate_source_directory_path(source_directory_path.as_ref())
        .map_err(MoveDirectoryPreparationError::SourceDirectoryValidationError)?;


    let validated_destination_directory = validate_destination_directory_path(
        destination_directory_path.as_ref(),
        options.destination_directory_rule,
    )
    .map_err(MoveDirectoryPreparationError::DestinationDirectoryValidationError)?;

    validate_source_destination_directory_pair(
        &validated_source_directory.directory_path,
        &validated_destination_directory.directory_path,
    )
    .map_err(MoveDirectoryPreparationError::DestinationDirectoryValidationError)?;


    let source_details =
        collect_source_directory_details(&validated_source_directory.directory_path)?;


    match attempt_directory_move_by_rename(
        &validated_source_directory,
        &source_details,
        &validated_destination_directory,
    )? {
        DirectoryMoveByRenameAction::Renamed { finished_move } => {
            return Ok(finished_move);
        }
        DirectoryMoveByRenameAction::Impossible => {}
    };


    // At this point a simple rename was either impossible or failed.
    // We need to copy and delete instead.

    let prepared_copy = DirectoryCopyPrepared::prepare_with_validated(
        validated_source_directory.clone(),
        validated_destination_directory,
        options.destination_directory_rule,
        CopyDirectoryDepthLimit::Unlimited,
    )
    .map_err(MoveDirectoryPreparationError::CopyPlanningError)?;

    copy_directory_unchecked(
        prepared_copy,
        CopyDirectoryOptions {
            destination_directory_rule: options.destination_directory_rule,
            copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
        },
    )
    .map_err(MoveDirectoryExecutionError::CopyDirectoryError)?;


    let directory_path_to_remove =
        if validated_source_directory.original_path_was_symlink_to_directory {
            source_directory_path.as_ref()
        } else {
            validated_source_directory.directory_path.as_path()
        };

    fs::remove_dir_all(directory_path_to_remove).map_err(|error| {
        MoveDirectoryExecutionError::UnableToAccessSource {
            path: validated_source_directory.directory_path,
            error,
        }
    })?;


    Ok(MoveDirectoryFinished {
        total_bytes_moved: source_details.total_bytes,
        files_moved: source_details.total_files,
        directories_moved: source_details.total_directories,
        strategy_used: DirectoryMoveStrategy::CopyAndDelete,
    })
}


/// Options that influence the [`move_directory_with_progress`] function.
pub struct MoveDirectoryWithProgressOptions {
    /// Specifies whether you allow the destination directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty destination directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Internal buffer size used for reading source files.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to a destination file.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
    ///
    /// *Note that the real reporting interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for MoveDirectoryWithProgressOptions {
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            // 64 KiB
            read_buffer_size: 1024 * 64,
            // 64 KiB
            write_buffer_size: 1024 * 64,
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
/// Depending on the [`options.destination_directory_rule`][DirectoryMoveOptions::destination_directory_rule] option,
/// the `target_directory_path` must:
/// - with [`DisallowExisting`][DestinationDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][DestinationDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][DestinationDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see [variant fields][DestinationDirectoryRule::AllowNonEmpty]).
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
/// (see [`MoveDirectoryFinished`]).
pub fn move_directory_with_progress<S, T, F>(
    source_directory_path: S,
    target_directory_path: T,
    options: MoveDirectoryWithProgressOptions,
    mut progress_handler: F,
) -> Result<MoveDirectoryFinished, MoveDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryMoveProgress),
{
    let validated_source_directory = validate_source_directory_path(source_directory_path.as_ref())
        .map_err(MoveDirectoryPreparationError::SourceDirectoryValidationError)?;

    let validated_destination_directory = validate_destination_directory_path(
        target_directory_path.as_ref(),
        options.destination_directory_rule,
    )
    .map_err(MoveDirectoryPreparationError::DestinationDirectoryValidationError)?;

    validate_source_destination_directory_pair(
        &validated_source_directory.directory_path,
        &validated_destination_directory.directory_path,
    )
    .map_err(MoveDirectoryPreparationError::DestinationDirectoryValidationError)?;


    let source_details =
        collect_source_directory_details(&validated_source_directory.directory_path)?;


    // We'll first attempt to move the directory by renaming it.
    // If we don't succeed (e.g. source and target paths are on different drives),
    // we'll copy and delete instead.


    match attempt_directory_move_by_rename(
        &validated_source_directory,
        &source_details,
        &validated_destination_directory,
    )? {
        DirectoryMoveByRenameAction::Renamed { finished_move } => {
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


            return Ok(finished_move);
        }
        DirectoryMoveByRenameAction::Impossible => {}
    };


    // At this point a simple rename was either impossible or failed.
    // We need to copy and delete instead.

    let copy_options = CopyDirectoryWithProgressOptions {
        destination_directory_rule: options.destination_directory_rule,
        read_buffer_size: options.read_buffer_size,
        write_buffer_size: options.write_buffer_size,
        progress_update_byte_interval: options.progress_update_byte_interval,
        copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
    };

    let prepared_copy = DirectoryCopyPrepared::prepare_with_validated(
        validated_source_directory.clone(),
        validated_destination_directory,
        copy_options.destination_directory_rule,
        copy_options.copy_depth_limit,
    )
    .map_err(MoveDirectoryPreparationError::CopyPlanningError)?;


    let directory_copy_result = execute_prepared_copy_directory_with_progress_unchecked(
        prepared_copy,
        copy_options,
        |progress| {
            let move_operation = match progress.current_operation.clone() {
                CopyDirectoryOperation::CreatingDirectory {
                    destination_directory_path: target_path,
                } => DirectoryMoveOperation::CreatingDirectory { target_path },
                CopyDirectoryOperation::CopyingFile {
                    destination_file_path: target_path,
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
        },
    )
    .map_err(MoveDirectoryExecutionError::CopyDirectoryError)?;


    // Having fully copied the directory to the target, we now
    // remove the original (source) directory.
    let directory_path_to_remove =
        if validated_source_directory.original_path_was_symlink_to_directory {
            source_directory_path.as_ref()
        } else {
            validated_source_directory.directory_path.as_path()
        };

    fs::remove_dir_all(directory_path_to_remove).map_err(|error| {
        MoveDirectoryExecutionError::UnableToAccessSource {
            path: validated_source_directory.directory_path,
            error,
        }
    })?;


    Ok(MoveDirectoryFinished {
        directories_moved: directory_copy_result.directories_created,
        total_bytes_moved: directory_copy_result.total_bytes_copied,
        files_moved: directory_copy_result.files_copied,
        strategy_used: DirectoryMoveStrategy::CopyAndDelete,
    })
}
