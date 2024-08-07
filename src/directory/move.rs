use std::path::{Path, PathBuf};

use_enabled_fs_module!();

use super::{
    collected::collect_directory_statistics_via_scan,
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
    BrokenSymlinkBehaviour,
    CopyDirectoryDepthLimit,
    DestinationDirectoryRule,
    DirectoryCopyOperation,
    DirectoryCopyOptions,
    DirectoryCopyWithProgressOptions,
    SymlinkBehaviour,
};
use crate::{
    error::{MoveDirectoryError, MoveDirectoryExecutionError, MoveDirectoryPreparationError},
    file::FileProgress,
    DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
    DEFAULT_READ_BUFFER_SIZE,
    DEFAULT_WRITE_BUFFER_SIZE,
};


// TODO implement, document and test
pub struct DirectoryMoveByCopyOptions {
    // TODO implement, document and test
    pub symlink_behaviour: SymlinkBehaviour,

    // TODO implement, document and test
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,
}

impl Default for DirectoryMoveByCopyOptions {
    fn default() -> Self {
        Self {
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Preserve,
        }
    }
}


// TODO implement, test, and document
pub enum DirectoryMoveAllowedStrategies {
    // TODO implement, test, and document
    OnlyRename,

    // TODO implement, test, and document
    OnlyCopyAndDelete {
        options: DirectoryMoveByCopyOptions,
    },

    // TODO implement, test, and document
    Either {
        copy_and_delete_options: DirectoryMoveByCopyOptions,
    },
}

impl DirectoryMoveAllowedStrategies {
    ///
    /// Returns `true` if the allowed strategies include moving by rename.
    ///
    /// # Invariants
    /// At least one of [`Self::into_options_if_may_copy_and_delete`] and [`Self::may_rename`] will always return `true` / `Some`.
    #[inline]
    pub(crate) fn allowed_to_rename(&self) -> bool {
        matches!(self, Self::OnlyRename | Self::Either { .. })
    }

    /// Returns `Some(`[`DirectoryMoveByCopyOptions`])` if the allowed strategies include moving by copy-and-delete,
    /// `None` otherwise.
    ///
    /// # Invariants
    /// At least one of [`Self::into_options_if_may_copy_and_delete`] and [`Self::may_rename`] will always return `true` / `Some`.
    #[inline]
    pub(crate) fn into_options_if_allowed_to_copy_and_delete(
        self,
    ) -> Option<DirectoryMoveByCopyOptions> {
        match self {
            DirectoryMoveAllowedStrategies::OnlyRename => None,
            DirectoryMoveAllowedStrategies::OnlyCopyAndDelete { options } => Some(options),
            DirectoryMoveAllowedStrategies::Either {
                copy_and_delete_options,
            } => Some(copy_and_delete_options),
        }
    }
}

impl Default for DirectoryMoveAllowedStrategies {
    fn default() -> Self {
        Self::Either {
            copy_and_delete_options: DirectoryMoveByCopyOptions::default(),
        }
    }
}


/// Options that influence the [`move_directory`] function.
pub struct DirectoryMoveOptions {
    /// Specifies whether you allow the target directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist
    /// (and whether you allow them to be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    // TODO implement, test, and document
    pub allowed_strategies: DirectoryMoveAllowedStrategies,
}

impl Default for DirectoryMoveOptions {
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            allowed_strategies: DirectoryMoveAllowedStrategies::default(),
        }
    }
}



/// Describes a strategy for performing a directory move.
///
/// This is included in [`DirectoryMoveFinished`] to allow
/// callers to understand how the directory was moved.
/// Note that *the caller can not request that a specific move strategy be used*.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectoryMoveStrategy {
    /// The source directory was simply renamed from the source path to the target path.
    ///
    /// **This is the fastest method**, to the point of being near instantaneous,
    /// but generally works only if both paths are on the same mount point or drive.
    Rename,

    /// The source directory was recursively copied to the target directory,
    /// and the source directory was deleted afterwards.
    ///
    /// This method is as fast as a normal recursive copy.
    /// It is also unavoidable if the directory can't renamed, which can happen when the source and destination
    /// directory exist on different mount points or drives.
    CopyAndDelete,
}



/// Describes actions taken by the [`move_directory`] function.
///
/// This is the return value of [`move_directory`] and [`move_directory_with_progress`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryMoveFinished {
    /// Total number of bytes moved.
    pub total_bytes_moved: u64,

    /// Number of files moved (details depend on strategy).
    pub files_moved: usize,

    /// Total number of symlinks moved (details depend on strategy).
    pub symlinks_moved: usize,

    /// Number of directories moved (details depend on strategy).
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

    /// Total number of symlinks in the directory (recursive).
    pub(crate) total_symlinks: usize,

    /// Total number of subdirectories in the directory (recursive).
    pub(crate) total_directories: usize,
}



/// Scans the provided directory for auxiliary details (without a depth limit).
/// This includes information like the total number of bytes it contains.
fn collect_source_directory_details(
    source_directory_path: &Path,
) -> Result<DirectoryContentDetails, MoveDirectoryPreparationError> {
    let directory_statistics = collect_directory_statistics_via_scan(source_directory_path)?;

    Ok(DirectoryContentDetails {
        total_bytes: directory_statistics.total_bytes,
        total_files: directory_statistics.total_files,
        total_symlinks: directory_statistics.total_symlinks,
        total_directories: directory_statistics.total_directories,
    })
}



pub(crate) enum DirectoryMoveByRenameAction {
    Renamed {
        finished_move: DirectoryMoveFinished,
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


    #[cfg(unix)]
    {
        // If the destination directory either does not exist or is empty,
        // a move by rename might be possible, but not otherwise.
        if !matches!(
            validated_destination_directory.state,
            DestinationDirectoryState::DoesNotExist | DestinationDirectoryState::IsEmpty
        ) {
            return Ok(DirectoryMoveByRenameAction::Impossible);
        }


        // Let's try to rename the source directory to the target.
        // This might still fail due to different mount points.
        if fs::rename(
            &validated_source_directory.unfollowed_directory_path,
            &validated_destination_directory.directory_path,
        )
        .is_ok()
        {
            return Ok(DirectoryMoveByRenameAction::Renamed {
                finished_move: DirectoryMoveFinished {
                    total_bytes_moved: source_directory_details.total_bytes,
                    files_moved: source_directory_details.total_files,
                    symlinks_moved: source_directory_details.total_symlinks,
                    directories_moved: source_directory_details.total_directories,
                    strategy_used: DirectoryMoveStrategy::Rename,
                },
            });
        }

        Ok(DirectoryMoveByRenameAction::Impossible)
    }

    #[cfg(windows)]
    {
        // If the destination directory does not exist,
        // a move by rename might be possible, but not otherwise.
        // This is because we're on Windows, where renames are only possible with non-existing destinations.
        if !matches!(
            validated_destination_directory.state,
            DestinationDirectoryState::DoesNotExist
        ) {
            return Ok(DirectoryMoveByRenameAction::Impossible);
        }


        // On Windows, the destination directory in call to `rename` must not exist for it to work.
        if !validated_destination_directory.state.exists()
            && fs::rename(
                &validated_source_directory.unfollowed_directory_path,
                &validated_destination_directory.directory_path,
            )
            .is_ok()
        {
            return Ok(DirectoryMoveByRenameAction::Renamed {
                finished_move: DirectoryMoveFinished {
                    total_bytes_moved: source_directory_details.total_bytes,
                    files_moved: source_directory_details.total_files,
                    symlinks_moved: source_directory_details.total_symlinks,
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
/// `source_directory_path` must point to an existing directory.
///
/// # Symbolic links
/// If `source_directory_path` is itself a symlink to a directory,
/// we'll try to move the link itself by renaming it to the destination.
/// If the rename fails, the link will be followed and not preserved
/// by performing a directory copy, after which the symlink will be removed.
///
/// For symlinks *inside* the source directory, the behaviour is different depending on the move strategy:
/// - If a move by rename succeeds, any symbolic links inside the source directory, valid or not, will be preserved.
/// - If the copy-and-delete fallback is used, all symbolic links are followed and not preserved
///   (see details in [`copy_directory`]).
///
///
/// # Options
/// See [`DirectoryMoveOptions`] for a full set of available directory moving options.
///
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged into the destination directory.
/// Note that this is not the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Move strategies
/// TODO update with strategy restriction options
///
/// Depending on the situation, the move can be performed one of two ways:
/// - The source directory can be simply renamed to the destination directory.
///   This is the preferred (and fastest) method, and will preserve
///   the `source_directory_path` symlink, if it is one.
///   In addition to some other platform-specifics<sup>*</sup>,
///   this strategy requires that the destination directory is empty or doesn't exist.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* Windows: the destination directory must not exist; if it does,
/// *even if it is empty*, the rename strategy will fail.</sup>
///
///
/// # Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved and how the move was performed
/// (see [`DirectoryMoveFinished`]).
///
///
///
/// <br>
///
/// #### See also
/// If you are looking for a directory moving function function that reports progress,
/// see [`move_directory_with_progress`].
///
///
/// [`copy_directory`]: super::copy_directory
/// [`options.destination_directory_rule`]: DirectoryMoveOptions::destination_directory_rule
/// [`DisallowExisting`]: DestinationDirectoryRule::DisallowExisting
/// [`AllowEmpty`]: DestinationDirectoryRule::AllowEmpty
pub fn move_directory<S, T>(
    source_directory_path: S,
    destination_directory_path: T,
    options: DirectoryMoveOptions,
) -> Result<DirectoryMoveFinished, MoveDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    // TODO update function documentation (regarding symlink options)

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


    if options.allowed_strategies.allowed_to_rename() {
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
    }


    let Some(copy_and_delete_options) = options
        .allowed_strategies
        .into_options_if_allowed_to_copy_and_delete()
    else {
        // This branch can execute only when a rename was attempted and failed,
        // and the user disabled the copy-and-delete fallback strategy.
        return Err(MoveDirectoryError::ExecutionError(
            MoveDirectoryExecutionError::RenameFailedAndNoFallbackStrategy,
        ));
    };


    // At this point a simple rename was either impossible or failed,
    // but the copy-and-delete fallback is enabled, so we should do that.
    let prepared_copy = DirectoryCopyPrepared::prepare_with_validated(
        validated_source_directory.clone(),
        validated_destination_directory,
        options.destination_directory_rule,
        CopyDirectoryDepthLimit::Unlimited,
        copy_and_delete_options.symlink_behaviour,
        copy_and_delete_options.broken_symlink_behaviour,
    )
    .map_err(MoveDirectoryPreparationError::CopyPlanningError)?;

    copy_directory_unchecked(
        prepared_copy,
        DirectoryCopyOptions {
            destination_directory_rule: options.destination_directory_rule,
            copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
            symlink_behaviour: copy_and_delete_options.symlink_behaviour,
            broken_symlink_behaviour: copy_and_delete_options.broken_symlink_behaviour,
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


    Ok(DirectoryMoveFinished {
        total_bytes_moved: source_details.total_bytes,
        files_moved: source_details.total_files,
        symlinks_moved: source_details.total_symlinks,
        directories_moved: source_details.total_directories,
        strategy_used: DirectoryMoveStrategy::CopyAndDelete,
    })
}




// TODO implement, document and test
pub struct DirectoryMoveWithProgressByCopyOptions {
    // TODO implement, document and test
    // TODO Note that changing this from Keep might make more moves possible, but would result in inconsistent behaviour
    //      between strategies, so only do it if you know what you are doing.
    pub symlink_behaviour: SymlinkBehaviour,

    // TODO implement, document and test
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,

    /// Internal buffer size used for reading source files.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to a destination file.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    ///
    /// Defaults to 512 KiB.
    ///
    /// *Note that the real reporting interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for DirectoryMoveWithProgressByCopyOptions {
    fn default() -> Self {
        Self {
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Preserve,
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            write_buffer_size: DEFAULT_WRITE_BUFFER_SIZE,
            progress_update_byte_interval: DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
        }
    }
}


// TODO implement, test, and document
pub enum DirectoryMoveWithProgressAllowedStrategies {
    // TODO implement, test, and document
    OnlyRename,

    // TODO implement, test, and document
    OnlyCopyAndDelete {
        options: DirectoryMoveWithProgressByCopyOptions,
    },

    // TODO implement, test, and document
    Either {
        copy_and_delete_options: DirectoryMoveWithProgressByCopyOptions,
    },
}

impl DirectoryMoveWithProgressAllowedStrategies {
    ///
    /// Returns `true` if the allowed strategies include moving by rename.
    ///
    /// # Invariants
    /// At least one of [`Self::into_options_if_may_copy_and_delete`] and [`Self::may_rename`] will always return `true` / `Some`.
    #[inline]
    pub(crate) fn allowed_to_rename(&self) -> bool {
        matches!(self, Self::OnlyRename | Self::Either { .. })
    }

    /// Returns `Some(`[`DirectoryMoveWithProgressByCopyOptions`])` if the allowed strategies include moving by copy-and-delete,
    /// `None` otherwise.
    ///
    /// # Invariants
    /// At least one of [`Self::into_options_if_may_copy_and_delete`] and [`Self::may_rename`] will always return `true` / `Some`.
    #[inline]
    pub(crate) fn into_options_if_allowed_to_copy_and_delete(
        self,
    ) -> Option<DirectoryMoveWithProgressByCopyOptions> {
        match self {
            DirectoryMoveWithProgressAllowedStrategies::OnlyRename => None,
            DirectoryMoveWithProgressAllowedStrategies::OnlyCopyAndDelete { options } => {
                Some(options)
            }
            DirectoryMoveWithProgressAllowedStrategies::Either {
                copy_and_delete_options,
            } => Some(copy_and_delete_options),
        }
    }
}

impl Default for DirectoryMoveWithProgressAllowedStrategies {
    fn default() -> Self {
        Self::Either {
            copy_and_delete_options: DirectoryMoveWithProgressByCopyOptions::default(),
        }
    }
}



/// Options that influence the [`move_directory_with_progress`] function.
pub struct DirectoryMoveWithProgressOptions {
    /// Specifies whether you allow the destination directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty destination directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    // TODO implement, test, and document
    pub allowed_strategies: DirectoryMoveWithProgressAllowedStrategies,
}

impl Default for DirectoryMoveWithProgressOptions {
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            allowed_strategies: DirectoryMoveWithProgressAllowedStrategies::default(),
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

    /// Describes a symbolic link being created.
    CreatingSymbolicLink {
        /// Path to the symlink being created.
        destination_symbolic_link_file_path: PathBuf,
    },

    /// Describes removal of the source directory.
    /// This happens at the very end when moving a directory.
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
    pub current_operation_index: usize,

    /// The total amount of operations that need to be performed to move the requested directory.
    ///
    /// A single operation is one of (see [`DirectoryMoveProgress`]):
    /// - copying a file,
    /// - creating a directory or
    /// - removing the source directory (at the very end).
    pub total_operations: usize,
}


/// Moves a directory from the source to the destination directory, with progress reporting.
///
/// `source_directory_path` must point to an existing directory.
///
/// # Symbolic links
/// If `source_directory_path` is itself a symlink to a directory,
/// we'll try to move the link itself by renaming it to the destination.
/// If the rename fails, the link will be followed and not preserved
/// by performing a directory copy, after which the symlink will be removed.
///
/// For symlinks *inside* the source directory, the behaviour is different depending on the move strategy:
/// - If a move by rename succeeds, any symbolic links inside the source directory, valid or not, will be preserved.
/// - If the copy-and-delete fallback is used, all symbolic links are followed and not preserved
///   (see details in [`copy_directory_with_progress`]).
///
///
///
/// # Options
/// See [`DirectoryMoveWithProgressOptions`] for a full set of available directory moving options.
///
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged into the destination directory.
/// Note that this is not the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Move strategies
/// TODO update with strategy restriction options
///
/// Depending on the situation, the move can be performed one of two ways:
/// - The source directory can be simply renamed to the destination directory.
///   This is the preferred (and fastest) method, and will preserve
///   the `source_directory_path` symlink, if it is one.
///   In addition to some other platform-specifics<sup>*</sup>,
///   this strategy requires that the destination directory is empty or doesn't exist.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* Windows: the destination directory must not exist; if it does,
/// *even if it is empty*, the rename strategy will fail.</sup>
///
///
/// # Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved and how the move was performed
/// (see [`DirectoryMoveFinished`]).
///
///
/// ### Progress reporting
/// This function allows you to receive progress reports by providing
/// a `progress_handler` closure. It will be called with
/// a reference to [`DirectoryMoveProgress`] regularly.
///
/// You can control the progress reporting frequency by setting the
/// [`progress_update_byte_interval`] option to a sufficiencly small or large value,
/// but note that smaller intervals are likely to have an additional impact on performance.
/// The value of this option if the minimum amount of bytes written to a file between
/// two calls to the provided `progress_handler`.
///
/// This function does not guarantee a precise amount of progress reports;
/// it does, however, guarantee at least one progress report per file and directory operation.
/// It also guarantees one final progress report, when the state indicates the move has been completed.
///
/// If the move can be performed by renaming the directory, only one progress report will be emitted.
///
///
/// <br>
///
/// #### See also
/// If you are looking for a directory moving function function that does not report progress,
/// see [`move_directory`].
///
///
/// [`copy_directory_with_progress`]: super::copy_directory_with_progress
/// [`options.destination_directory_rule`]: DirectoryMoveWithProgressOptions::destination_directory_rule
/// [`progress_update_byte_interval`]: DirectoryMoveWithProgressByCopyOptions::progress_update_byte_interval
/// [`DisallowExisting`]: DestinationDirectoryRule::DisallowExisting
/// [`AllowEmpty`]: DestinationDirectoryRule::AllowEmpty
pub fn move_directory_with_progress<S, T, F>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryMoveWithProgressOptions,
    mut progress_handler: F,
) -> Result<DirectoryMoveFinished, MoveDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryMoveProgress),
{
    // TODO update function documentation (regarding symlink options)

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


    if options.allowed_strategies.allowed_to_rename() {
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
    }

    let Some(copy_and_delete_options) = options
        .allowed_strategies
        .into_options_if_allowed_to_copy_and_delete()
    else {
        // This branch can execute only when a rename was attempted and failed,
        // and the user disabled the copy-and-delete fallback strategy.
        return Err(MoveDirectoryError::ExecutionError(
            MoveDirectoryExecutionError::RenameFailedAndNoFallbackStrategy,
        ));
    };


    // At this point a simple rename was either impossible or failed.
    // We need to copy and delete instead.

    let copy_options = DirectoryCopyWithProgressOptions {
        destination_directory_rule: options.destination_directory_rule,
        read_buffer_size: copy_and_delete_options.read_buffer_size,
        write_buffer_size: copy_and_delete_options.write_buffer_size,
        progress_update_byte_interval: copy_and_delete_options.progress_update_byte_interval,
        copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
        symlink_behaviour: copy_and_delete_options.symlink_behaviour,
        broken_symlink_behaviour: copy_and_delete_options.broken_symlink_behaviour,
    };

    let prepared_copy = DirectoryCopyPrepared::prepare_with_validated(
        validated_source_directory.clone(),
        validated_destination_directory,
        copy_options.destination_directory_rule,
        copy_options.copy_depth_limit,
        copy_and_delete_options.symlink_behaviour,
        copy_and_delete_options.broken_symlink_behaviour,
    )
    .map_err(MoveDirectoryPreparationError::CopyPlanningError)?;


    let directory_copy_result = execute_prepared_copy_directory_with_progress_unchecked(
        prepared_copy,
        copy_options,
        |progress| {
            let move_operation = match progress.current_operation.clone() {
                DirectoryCopyOperation::CreatingDirectory {
                    destination_directory_path: target_path,
                } => DirectoryMoveOperation::CreatingDirectory { target_path },
                DirectoryCopyOperation::CopyingFile {
                    destination_file_path: target_path,
                    progress,
                } => DirectoryMoveOperation::CopyingFile {
                    target_path,
                    progress,
                },
                DirectoryCopyOperation::CreatingSymbolicLink {
                    destination_symbolic_link_file_path,
                } => DirectoryMoveOperation::CreatingSymbolicLink {
                    destination_symbolic_link_file_path,
                },
            };


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


    Ok(DirectoryMoveFinished {
        directories_moved: directory_copy_result.directories_created,
        total_bytes_moved: directory_copy_result.total_bytes_copied,
        files_moved: directory_copy_result.files_copied,
        symlinks_moved: directory_copy_result.symlinks_created,
        strategy_used: DirectoryMoveStrategy::CopyAndDelete,
    })
}
