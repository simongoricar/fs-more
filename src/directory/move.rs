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
    DestinationDirectoryRule,
    DirectoryCopyDepthLimit,
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


/// Options for the copy-and-delete strategy when moving a directory.
///
/// See also: [`DirectoryMoveOptions`] and [`move_directory`].
pub struct DirectoryMoveByCopyOptions {
    /// Sets the behaviour for symbolic links when moving a directory by copy-and-delete.
    ///
    /// Note that setting this to [`SymlinkBehaviour::Follow`] instead of
    /// [`SymlinkBehaviour::Keep`] (keep is the default) will result in behaviour
    /// that differs than the rename method (that one will always keep symbolic links).
    /// In other words, if both strategies are enabled and this is changed from the default,
    /// you will need to look at which strategy was used after the move to discern
    /// whether symbolic links were actually preserved or not.
    ///
    /// This has the same impact as the [`symlink_behaviour`][dco-symlink_behaviour]
    /// option under [`DirectoryCopyOptions`].
    ///
    ///
    /// [dco-symlink_behaviour]: crate::directory::DirectoryCopyOptions::symlink_behaviour
    pub symlink_behaviour: SymlinkBehaviour,

    /// Sets the behaviour for broken symbolic links when moving a directory by copy-and-delete.
    ///
    /// This has the same impact as the [`broken_symlink_behaviour`][dco-broken_symlink_behaviour]
    /// option under [`DirectoryCopyOptions`].
    ///
    ///
    /// [dco-broken_symlink_behaviour]: crate::directory::DirectoryCopyOptions::broken_symlink_behaviour
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,
}

impl Default for DirectoryMoveByCopyOptions {
    /// Initializes the default options for the copy-and-delete strategy when moving a directory:
    /// - symbolic links are kept, and
    /// - broken symbolic links are preserved as-is (i.e. kept broken).
    fn default() -> Self {
        Self {
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Keep,
        }
    }
}


/// Describes the allowed strategies for moving a directory.
///
/// This ensures at least one of "rename" or "copy-and-delete" strategies are enabled at any point.
/// Unless you have a good reason for picking something else, [`Self::Either`]
/// is highly recommended. It ensures we always try to rename the directory if the
/// conditions are right, and fall back to the slower copy-and-delete strategy if that fails.
///
/// See also: [`DirectoryMoveOptions`] and [`move_directory`].
pub enum DirectoryMoveAllowedStrategies {
    /// Disables the move by copy-and-delete strategy, leaving only the rename strategy.
    ///
    /// If renaming fails, for example due to source and destination being on different mount points,
    /// the corresponding function will return
    /// [`ExecutionError`]`(`[`RenameFailedAndNoFallbackStrategy`]`)`.
    ///
    ///
    /// [`ExecutionError`]: crate::error::MoveDirectoryError::ExecutionError
    /// [`RenameFailedAndNoFallbackStrategy`]: crate::error::MoveDirectoryExecutionError::RenameFailedAndNoFallbackStrategy
    OnlyRename,

    /// Disables the move by rename strategy, leaving only the less efficient,
    /// but more general, copy-and-delete strategy.
    OnlyCopyAndDelete {
        /// Options for the copy-and-delete strategy.
        options: DirectoryMoveByCopyOptions,
    },

    /// Enables both the rename and copy-and-delete strategies,
    /// leaving the optimal choice in the hands of the library.
    ///
    /// Generally speaking, a rename will be attempted under the right conditions,
    /// with the copy-and-delete performed as a fallback if the rename fails.
    Either {
        /// Options for the copy-and-delete strategy.
        copy_and_delete_options: DirectoryMoveByCopyOptions,
    },
}

impl DirectoryMoveAllowedStrategies {
    /// Returns `true` if the allowed move strategies include moving by rename.
    ///
    /// # Invariants
    /// At least one of [`Self::allowed_to_rename`] and [`Self::into_options_if_may_copy_and_delete`]
    /// will always return `true` or `Some(...)`, respectively.
    #[inline]
    pub(crate) fn allowed_to_rename(&self) -> bool {
        matches!(self, Self::OnlyRename | Self::Either { .. })
    }

    /// Returns `Some(`[`DirectoryMoveByCopyOptions`])` if the allowed move strategies include moving by copy-and-delete,
    /// and returns `None` otherwise.
    ///
    /// # Invariants
    /// At least one of [`Self::allowed_to_rename`] and [`Self::into_options_if_may_copy_and_delete`]
    /// will always return `true` or `Some(...)`, respectively.
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
    /// Returns the default directory move strategy configuration,
    /// which is with both rename and copy-and-delete enabled.
    ///
    /// For details on the default copy-and-delete options,
    /// see [`DirectoryMoveByCopyOptions::default`].
    fn default() -> Self {
        Self::Either {
            copy_and_delete_options: DirectoryMoveByCopyOptions::default(),
        }
    }
}


/// Options that influence the [`move_directory`] function.
///
/// ## `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is not the default, and you should probably consider the consequences
/// very carefully before using that option.
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

    /// Sets the allowed directory move strategies.
    /// Per-strategy options are also configured here.
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



/// Describes a strategy usef when a directory move was performed.
///
/// This is included in [`DirectoryMoveFinished`] to allow
/// callers to understand how the directory was moved.
///
/// This is used only as a return value; if you want to control the
/// available directory move strategies, see [`DirectoryMoveAllowedStrategies`]
/// and the options described in [`move_directory`] / [`move_directory_with_progress`].
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


/// Describes the result of a [`attempt_directory_move_by_rename`] call,
/// signalling whether the rename succeeded or not.
pub(crate) enum DirectoryMoveByRenameAction {
    /// The directory was successfully moved by renaming it to the destination.
    Renamed {
        /// Details of the finished directory move.
        finished_move: DirectoryMoveFinished,
    },

    /// The directory could not be moved by renaming it,
    /// be it either due to the destination being non-empty or due to
    /// failing the actual directory rename call.
    FailedOrImpossible,
}


/// Attempts a directory move by using [`std::fs::rename`]
/// (or `fs_err::rename` if the `fs-err` feature flag is enabled).
///
/// Returns [`DirectoryMoveByRenameAction`], which indicates whether the move by rename
/// succeeded or failed due to source and destination being on different mount points or drives.
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
            return Ok(DirectoryMoveByRenameAction::FailedOrImpossible);
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

        Ok(DirectoryMoveByRenameAction::FailedOrImpossible)
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
            return Ok(DirectoryMoveByRenameAction::FailedOrImpossible);
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

        Ok(DirectoryMoveByRenameAction::FailedOrImpossible)
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
/// For symlinks *inside* the source directory, the behaviour is different depending on the move strategy
/// (individual strategies can be disabled, see section below):
/// - If the destination is non-existent (or empty), a move by rename will be attempted first.
///   In that case, any symbolic links inside the source directory, valid or not, will be preserved.
/// - If the copy-and-delete fallback is used, the behaviour depends on the [`symlink_behaviour`]
///   option for that particular strategy (the default is to keep symbolic links as-is).
///
///
/// # Options
/// See [`DirectoryMoveOptions`] for a full set of available directory moving options.
/// Note that certain strategy-specific options, such as copy-and-delete settings,
/// are available under the [`allowed_strategies`] options field
/// (see e.g. [`DirectoryMoveAllowedStrategies::Either`]).
///
/// ### `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is *not* the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Move strategies
/// The move can be performed using either of the two available strategies:
/// - The source directory can be simply renamed to the destination directory.
///   This is the preferred (and fastest) method. Additionally, if `source_directory_path` is itself
///   a symbolic link it has the side effect of preserving that.
///   This strategy requires that the destination directory is either empty or doesn't exist,
///   though precise conditions depend on platform<sup>*</sup>.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// **By default, a rename is attempted first, with copy-and-delete available as a fallback.**
/// Either of these strategies can be disabled in the options struct (see [`allowed_strategies`]),
/// but at least one must always be enabled.
///
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* Windows: the destination directory must not exist at all; if it does,
/// *even if it is empty*, the rename strategy will fail.</sup>
///
///
/// # Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total number of bytes moved and how the move was performed
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
/// [`symlink_behaviour`]: DirectoryMoveByCopyOptions::symlink_behaviour
/// [`allowed_strategies`]: DirectoryMoveOptions::allowed_strategies
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
            DirectoryMoveByRenameAction::FailedOrImpossible => {}
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
        DirectoryCopyDepthLimit::Unlimited,
        copy_and_delete_options.symlink_behaviour,
        copy_and_delete_options.broken_symlink_behaviour,
    )
    .map_err(MoveDirectoryPreparationError::CopyPlanningError)?;

    copy_directory_unchecked(
        prepared_copy,
        DirectoryCopyOptions {
            destination_directory_rule: options.destination_directory_rule,
            copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
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




/// Options for the copy-and-delete strategy when
/// configuring a directory move with progress tracking.
///
/// See also: [`DirectoryMoveWithProgressOptions`] and [`move_directory_with_progress`].
pub struct DirectoryMoveWithProgressByCopyOptions {
    /// Sets the behaviour for symbolic links when moving a directory by copy-and-delete.
    ///
    /// Note that setting this to [`SymlinkBehaviour::Follow`] instead of
    /// [`SymlinkBehaviour::Keep`] (keep is the default) will result in behaviour
    /// that differs than the rename method (that one will always keep symbolic links).
    /// In other words, if both strategies are enabled and this is changed from the default,
    /// you will need to look at which strategy was used after the move to discern
    /// whether symbolic links were actually preserved or not.
    ///
    /// This has the same impact as the [`symlink_behaviour`][dco-symlink_behaviour] option
    /// under [`DirectoryCopyWithProgressOptions`].
    ///
    ///
    /// [dco-symlink_behaviour]: crate::directory::DirectoryCopyWithProgressOptions::symlink_behaviour
    pub symlink_behaviour: SymlinkBehaviour,

    /// Sets the behaviour for broken symbolic links when moving a directory by copy-and-delete.
    ///
    /// This has the same impact as the [`broken_symlink_behaviour`][dco-broken_symlink_behaviour] option
    /// under [`DirectoryCopyWithProgressOptions`].
    ///
    ///
    /// [dco-broken_symlink_behaviour]: crate::directory::DirectoryCopyWithProgressOptions::broken_symlink_behaviour
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,

    /// Internal buffer size used for reading from source files.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to destination files.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// *Minimum* number of bytes written between two consecutive progress reports.
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
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Keep,
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            write_buffer_size: DEFAULT_WRITE_BUFFER_SIZE,
            progress_update_byte_interval: DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
        }
    }
}


/// Describes the allowed strategies for moving a directory
/// (with progress tracking).
///
/// This ensures at least one of "rename" or "copy-and-delete" strategies are enabled at any point.
/// Unless you have a good reason for picking something else, [`Self::Either`]
/// is highly recommended. It ensures we always try to rename the directory if the
/// conditions are right, and fall back to the slower copy-and-delete strategy if that fails.
///
/// See also: [`DirectoryMoveWithProgressOptions`] and [`move_directory_with_progress`].
pub enum DirectoryMoveWithProgressAllowedStrategies {
    /// Disables the move by copy-and-delete strategy, leaving only the rename strategy.
    ///
    /// If renaming fails, for example due to source and destination being on different
    /// mount points, the corresponding function will return
    /// [`ExecutionError`]`(`[`RenameFailedAndNoFallbackStrategy`]`)`.
    ///
    ///
    /// [`ExecutionError`]: crate::error::MoveDirectoryError::ExecutionError
    /// [`RenameFailedAndNoFallbackStrategy`]: crate::error::MoveDirectoryExecutionError::RenameFailedAndNoFallbackStrategy
    OnlyRename,

    /// Disables the move by rename strategy, leaving only the less efficient,
    /// but more general, copy-and-delete strategy.
    OnlyCopyAndDelete {
        /// Options for the copy-and-delete strategy.
        options: DirectoryMoveWithProgressByCopyOptions,
    },

    /// Enables both the rename and copy-and-delete strategies,
    /// leaving the optimal choice in the hands of the library.
    ///
    /// Generally speaking, a rename will be attempted under the right conditions,
    /// with the copy-and-delete performed as a fallback if the rename fails.
    Either {
        /// Options for the copy-and-delete strategy.
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
    /// Returns the default directory move strategy configuration,
    /// which is with both rename and copy-and-delete enabled.
    ///
    /// For details on the default copy-and-delete options,
    /// see [`DirectoryMoveWithProgressByCopyOptions::default`].
    fn default() -> Self {
        Self::Either {
            copy_and_delete_options: DirectoryMoveWithProgressByCopyOptions::default(),
        }
    }
}



/// Options that influence the [`move_directory_with_progress`] function.
///
/// ## `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is not the default, and you should probably consider the consequences
/// very carefully before using that option.
pub struct DirectoryMoveWithProgressOptions {
    /// Specifies whether you allow the destination directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty destination directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Sets the allowed directory move strategies.
    /// Per-strategy options are also configured here.
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
    /// Number of bytes that need to be moved for the directory move to be complete.
    pub bytes_total: u64,

    /// Number of bytes that have been moved so far.
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

    /// The total number of operations that need to be performed to move the requested directory.
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
/// For symlinks *inside* the source directory, the behaviour is different depending on the move strategy
/// (individual strategies can be disabled, see section below):
/// - If the destination is non-existent (or empty), a move by rename will be attempted first.
///   In that case, any symbolic links inside the source directory, valid or not, will be preserved.
/// - If the copy-and-delete fallback is used, the behaviour depends on the [`symlink_behaviour`]
///   option for that particular strategy (the default is to keep symbolic links as-is).
///
///
/// # Options
/// See [`DirectoryMoveWithProgressOptions`] for a full set of available directory moving options.
/// Note that certain strategy-specific options, such as copy-and-delete settings,
/// are available under the [`allowed_strategies`] options field
/// (see e.g. [`DirectoryMoveWithProgressAllowedStrategies::Either`]).
///
/// ### `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is *not* the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Move strategies
/// The move can be performed using either of the two available strategies:
/// - The source directory can be simply renamed to the destination directory.
///   This is the preferred (and fastest) method. Additionally, if `source_directory_path` is itself
///   a symbolic link it has the side effect of preserving that.
///   This strategy requires that the destination directory is either empty or doesn't exist,
///   though precise conditions depend on platform<sup>*</sup>.
/// - If the directory can't be renamed, the function will fall back to a copy-and-rename strategy.
///
/// **By default, a rename is attempted first, with copy-and-delete available as a fallback.**
/// Either of these strategies can be disabled in the options struct (see [`allowed_strategies`]),
/// but at least one must always be enabled.
///
///
/// For more information, see [`DirectoryMoveStrategy`].
///
/// <sup>* Windows: the destination directory must not exist at all; if it does,
/// *even if it is empty*, the rename strategy will fail.</sup>
///
///
/// # Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total number of bytes moved and how the move was performed
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
/// The value of this option if the minimum number of bytes written to a file between
/// two calls to the provided `progress_handler`.
///
/// This function does not guarantee a precise number of progress reports;
/// it does, however, guarantee at least one progress report per file copy, symlink and directory operation.
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
/// [`symlink_behaviour`]: DirectoryMoveWithProgressByCopyOptions::symlink_behaviour
/// [`allowed_strategies`]: DirectoryMoveWithProgressOptions::allowed_strategies
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
            DirectoryMoveByRenameAction::FailedOrImpossible => {}
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
        copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
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
