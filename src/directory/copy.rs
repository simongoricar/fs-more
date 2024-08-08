use std::path::{Path, PathBuf};

use_enabled_fs_module!();

use super::{
    common::DestinationDirectoryRule,
    prepared::{try_exists_without_follow, DirectoryCopyPrepared, QueuedOperation},
};
use crate::{
    error::{CopyDirectoryError, CopyDirectoryExecutionError},
    file::{
        copy_file,
        copy_file_with_progress,
        CollidingFileBehaviour,
        FileCopyOptions,
        FileCopyWithProgressOptions,
        FileProgress,
    },
    DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
    DEFAULT_READ_BUFFER_SIZE,
    DEFAULT_WRITE_BUFFER_SIZE,
};


/// The maximum depth of a directory copy operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectoryCopyDepthLimit {
    /// No depth limit - the entire directory tree will be copied.
    Unlimited,

    /// Copy depth is limited to `maximum_depth`, where the value refers to
    /// the maximum depth of the subdirectory whose contents are still copied.
    ///
    ///
    /// # Examples
    /// `maximum_depth = 0` indicates a copy operation that will cover only the files and directories
    /// directly in the source directory.
    ///
    /// ```md
    /// ~/source-directory
    ///  |- foo.csv
    ///  |- foo-2.csv
    ///  |- bar/
    ///     (no entries)
    /// ```
    ///
    /// Note that the `~/source-directory/bar` directory will still be created,
    /// but the corresponding files inside it in the source directory won't be copied.
    ///
    ///
    /// <br>
    ///
    /// `maximum_depth = 1` will cover the files and directories directly in the source directory
    /// plus one level of files and subdirectories deeper.
    ///
    /// ```md
    /// ~/source-directory
    ///  |- foo.csv
    ///  |- foo-2.csv
    ///  |- bar/
    ///     |- hello-world.txt
    ///     |- bar2/
    ///        (no entries)
    /// ```
    ///
    /// Notice how direct contents of `~/source-directory` and `~/source-directory/bar` are copied,
    /// but `~/source-directory/bar/bar2` is created, but stays empty on the destination.
    Limited {
        /// Maximum copy depth.
        maximum_depth: usize,
    },
}



/// How to behave when encountering symbolic links during directory copies or moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymlinkBehaviour {
    /// Indicates the symbolic link should be preserved on the destination.
    ///
    /// It is possible that a symbolic link cannot be created on the destination,
    /// for example in certain cases when source and destination are on different
    /// mount points, in which case an error will be returned.
    ///
    /// In this mode, broken symbolic links will be handled with the
    /// active [`BrokenSymlinkBehaviour`] option used alongside it.
    Keep,

    /// Indicates the symbolic link should be resolved and its destination content
    /// should be copied or moved to the destination instead of preserving the symbolic link.
    ///
    /// In this mode, broken symbolic links will always cause errors,
    /// regardless of the active [`BrokenSymlinkBehaviour`].
    Follow,
}



/// How to behave when encountering broken symbolic links during directory copies or moves.
///
/// This option is generally available alongside [`SymlinkBehaviour`].
/// Note that [`BrokenSymlinkBehaviour`] options are only used when
/// [`SymlinkBehaviour::Keep`] is used.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BrokenSymlinkBehaviour {
    /// Indicates that the broken symbolic link should be kept as-is on the destination, i.e. broken.
    ///
    /// Just like for normal symbolic links,
    /// it is possible that a symbolic link cannot be created on the destination —
    /// for example in certain cases when source and destination are on different
    /// mount points — in which case an error will be returned.
    Keep,

    /// Indicates a broken symbolic link should result in an error while preparing the copy or move.
    ///
    /// Note that unless symbolic link following is enabled alongside this option,
    /// [`BrokenSymlinkBehaviour::Abort`] will have no effect.
    Abort,
}



/// Options that influence the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyOptions {
    /// Specifies whether you allow the destination directory to exist before copying
    /// and whether it must be empty or not.
    /// If you allow a non-empty destination directory, you may also specify
    /// how to behave for existing destination files and sub-directories.
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Maximum depth of the source directory to copy over to the destination.
    pub copy_depth_limit: DirectoryCopyDepthLimit,

    /// Sets the behaviour for symbolic links when copying a directory.
    pub symlink_behaviour: SymlinkBehaviour,

    /// Sets the behaviour for broken symbolic links when copying a directory.
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,
}

impl Default for DirectoryCopyOptions {
    /// Constructs defaults for copying a directory, which are:
    /// - [`DestinationDirectoryRule::AllowEmpty`]: if the destination directory already exists, it must be empty,
    /// - [`DirectoryCopyDepthLimit::Unlimited`]: there is no copy depth limit,
    /// - [`SymlinkBehaviour::Keep`]: symbolic links are not followed, and
    /// - [`BrokenSymlinkBehaviour::Keep`]: broken symbolic links are kept as-is, i.e. broken.
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Keep,
        }
    }
}


/// Describes a successful directory copy operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyFinished {
    /// Total amount of bytes copied.
    pub total_bytes_copied: u64,

    /// Total number of files copied.
    pub files_copied: usize,

    /// Total number of symlinks (re)created.
    ///
    /// If the [`DirectoryCopyOptions::symlink_behaviour`] option is set to
    /// [`SymlinkBehaviour::Follow`], this will always be `0`.
    pub symlinks_created: usize,

    /// Total number of directories created.
    pub directories_created: usize,
}



/// Perform a copy using prepared data from [`DirectoryCopyPrepared`].
///
/// For more details, see [`copy_directory`].
pub(crate) fn copy_directory_unchecked(
    prepared_directory_copy: DirectoryCopyPrepared,
    options: DirectoryCopyOptions,
) -> Result<DirectoryCopyFinished, CopyDirectoryExecutionError> {
    let can_overwrite_files = options
        .destination_directory_rule
        .allows_overwriting_existing_destination_files();

    let can_ignore_existing_sub_directories = options
        .destination_directory_rule
        .allows_existing_destination_subdirectories();


    // We have the entire queue of operations, and we've made sure there are
    // no collisions we should worry about. What's left is performing the file copy
    // and directory creation operations *precisely in the order they have been prepared*.
    // If we ignore the order, we could get into situations where
    // some destination directory doesn't exist yet, but we would want to copy a file into it.


    let mut total_bytes_copied = 0;
    let mut num_files_copied = 0;
    let mut num_symlinks_recreated = 0;
    let mut num_directories_created = 0;


    // Create base destination directory if needed.
    let destination_directory_exists = prepared_directory_copy
        .validated_destination_directory
        .state
        .exists();

    if !destination_directory_exists {
        fs::create_dir_all(
            &prepared_directory_copy
                .validated_destination_directory
                .directory_path,
        )
        .map_err(|error| CopyDirectoryExecutionError::UnableToCreateDirectory {
            directory_path: prepared_directory_copy
                .validated_destination_directory
                .directory_path,
            error,
        })?;

        num_directories_created += 1;
    }


    // Execute all queued operations (copying files and creating directories).
    for operation in prepared_directory_copy.operation_queue {
        match operation {
            QueuedOperation::CopyFile {
                source_file_path,
                source_size_bytes,
                destination_file_path,
            } => {
                let destination_file_exists = try_exists_without_follow(&destination_file_path)
                    .map_err(|error| CopyDirectoryExecutionError::UnableToAccessDestination {
                        path: destination_file_path.clone(),
                        error,
                    })?;

                if destination_file_exists {
                    let destination_file_metadata = fs::symlink_metadata(&destination_file_path)
                        .map_err(|error| {
                            CopyDirectoryExecutionError::UnableToAccessDestination {
                                path: destination_file_path.clone(),
                                error,
                            }
                        })?;


                    if !destination_file_metadata.is_file() {
                        return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                            path: destination_file_path.clone(),
                        });
                    }

                    if !can_overwrite_files {
                        return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                            path: destination_file_path.clone(),
                        });
                    }
                }


                copy_file(
                    source_file_path,
                    &destination_file_path,
                    FileCopyOptions {
                        colliding_file_behaviour: match can_overwrite_files {
                            true => CollidingFileBehaviour::Overwrite,
                            false => CollidingFileBehaviour::Abort,
                        },
                    },
                )
                .map_err(|file_error| {
                    CopyDirectoryExecutionError::FileCopyError {
                        file_path: destination_file_path,
                        error: file_error,
                    }
                })?;


                num_files_copied += 1;
                total_bytes_copied += source_size_bytes;
            }

            QueuedOperation::CreateDirectory {
                source_size_bytes,
                destination_directory_path,
            } => {
                let destination_directory_exists =
                    try_exists_without_follow(&destination_directory_path).map_err(|error| {
                        CopyDirectoryExecutionError::UnableToAccessDestination {
                            path: destination_directory_path.clone(),
                            error,
                        }
                    })?;


                if destination_directory_exists {
                    if !destination_directory_path.is_dir() {
                        return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                            path: destination_directory_path.clone(),
                        });
                    }

                    if !can_ignore_existing_sub_directories {
                        return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                            path: destination_directory_path.clone(),
                        });
                    }

                    continue;
                }

                fs::create_dir(&destination_directory_path).map_err(|error| {
                    CopyDirectoryExecutionError::UnableToCreateDirectory {
                        directory_path: destination_directory_path,
                        error,
                    }
                })?;


                num_directories_created += 1;
                total_bytes_copied += source_size_bytes;
            }

            #[cfg(windows)]
            QueuedOperation::CreateSymlink {
                symlink_path,
                symlink_type,
                source_symlink_size_bytes,
                symlink_destination_path,
            } => {
                use crate::directory::prepared::SymlinkType;

                match symlink_type {
                    SymlinkType::File => {
                        std::os::windows::fs::symlink_file(
                            &symlink_destination_path,
                            &symlink_path,
                        )
                        .map_err(|error| {
                            CopyDirectoryExecutionError::SymlinkCreationError {
                                symlink_path: symlink_path.clone(),
                                error,
                            }
                        })?;
                    }
                    SymlinkType::Directory => {
                        std::os::windows::fs::symlink_dir(&symlink_destination_path, &symlink_path)
                            .map_err(|error| CopyDirectoryExecutionError::SymlinkCreationError {
                                symlink_path: symlink_path.clone(),
                                error,
                            })?;
                    }
                }


                num_symlinks_recreated += 1;
                total_bytes_copied += source_symlink_size_bytes;
            }

            #[cfg(unix)]
            QueuedOperation::CreateSymlink {
                symlink_path,
                source_symlink_size_bytes,
                symlink_destination_path,
            } => {
                std::os::unix::fs::symlink(&symlink_destination_path, &symlink_path).map_err(
                    |error| CopyDirectoryExecutionError::SymlinkCreationError {
                        symlink_path: symlink_path.clone(),
                        error,
                    },
                )?;


                num_symlinks_recreated += 1;
                total_bytes_copied += source_symlink_size_bytes;
            }
        };
    }


    Ok(DirectoryCopyFinished {
        total_bytes_copied,
        files_copied: num_files_copied,
        symlinks_created: num_symlinks_recreated,
        directories_created: num_directories_created,
    })
}


/// Copies a directory from the source to the destination directory.
///
/// Contents of the source directory will be copied into the destination directory.
/// If needed, the destination directory will be created before copying begins.
///
///
/// # Symbolic links
/// TODO the paragraph below is true, but we should match it to symlink_behaviour instead.
///      once done, copy to copy_directory_with_progress as well, then update relevant docs in move_directory
///
/// If the provided `source_directory_path` is itself a symlink that points to a directory,
/// the link will be followed and the contents of the link target directory will be copied.
///
/// Regarding symbolic links *inside* the source directory, the chosen [`symlink_behaviour`] is respected.
///
/// This matches the behaviour of `cp` with `--recursive` (and optionally `--dereference`)
/// flags on Unix[^unix-cp-rd].
///
///
/// # Options
/// See [`DirectoryCopyOptions`] for the full set of available directory copying options.
///
/// ### `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is *not* the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`DirectoryCopyFinished`].
///
///
/// # Errors
/// If the directory cannot be copied to the destination, a [`CopyDirectoryError`] is returned;
/// see its documentation for more details.
///
/// Errors for this function are quite granular, and are split into two main groups:
/// - Preparation errors ([`CopyDirectoryError::PreparationError`]) are emitted during
///   the preparation phase of copying. Importantly, if an error from this group is returned,
///   the destination directory *hasn't been changed yet* in any way.
/// - Copy execution errors ([`CopyDirectoryError::ExecutionError`]) are emitted during
///   the actual copying phase. If an error from this group is returned,
///   it is very likely that the destination directory is in an unpredictable state, since
///   the error occurred while trying to copy a file or create a directory.
///
///
/// [`options.destination_directory_rule`]: DirectoryCopyOptions::destination_directory_rule
/// [`options.copy_depth_limit`]: DirectoryCopyOptions::copy_depth_limit
/// [`symlink_behaviour`]: DirectoryCopyOptions::symlink_behaviour
/// [`DisallowExisting`]: DestinationDirectoryRule::DisallowExisting
/// [`AllowEmpty`]: DestinationDirectoryRule::AllowEmpty
/// [`AllowNonEmpty`]: DestinationDirectoryRule::AllowNonEmpty
/// [`copy_file`]: crate::file::copy_file
/// [^unix-cp-rd]: Source for coreutils' `cp` is available
///     [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/cp.c).
pub fn copy_directory<S, T>(
    source_directory_path: S,
    destination_directory_path: T,
    options: DirectoryCopyOptions,
) -> Result<DirectoryCopyFinished, CopyDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let prepared_copy = DirectoryCopyPrepared::prepare(
        source_directory_path.as_ref(),
        destination_directory_path.as_ref(),
        options.destination_directory_rule,
        options.copy_depth_limit,
        options.symlink_behaviour,
        options.broken_symlink_behaviour,
    )?;

    let finished_copy = copy_directory_unchecked(prepared_copy, options)?;

    Ok(finished_copy)
}


/// Describes a directory copy operation.
///
/// Used for progress reporting in [`copy_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DirectoryCopyOperation {
    /// A directory is being created.
    CreatingDirectory {
        /// Path to the directory that is being created.
        destination_directory_path: PathBuf,
    },

    /// A file is being copied.
    ///
    /// For more precise copying progress, see the `progress` field.
    CopyingFile {
        /// Path to the file that is being created.
        destination_file_path: PathBuf,

        /// Progress of the file copy operation.
        progress: FileProgress,
    },

    /// A symbolic link is being created.
    CreatingSymbolicLink {
        /// Path to the symlink being created.
        destination_symbolic_link_file_path: PathBuf,
    },
}


/// Directory copying progress.
///
/// This struct is used to report progress to a user-provided closure
/// (see usage in [`copy_directory_with_progress`]).
///
/// Note that the data inside this struct isn't fully owned - the `current_operation`
/// field is borrowed, and cloning will not have the desired effect.
/// To obtain a fully-owned clone of this state, call
/// [`DirectoryCopyProgressRef::to_owned_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryCopyProgressRef<'o> {
    /// Total number of bytes that need to be copied
    /// for the directory copy to be complete.
    pub bytes_total: u64,

    /// Number of bytes that have been copied so far.
    pub bytes_finished: u64,

    /// Number of files that have been copied so far.
    pub files_copied: usize,

    /// Number of symlinks that have been (re)created so far.
    ///
    /// If the [`DirectoryCopyOptions::symlink_behaviour`] option is set to
    /// [`SymlinkBehaviour::Follow`], this will always be `0`.
    pub symlinks_created: usize,

    /// Number of directories that have been created so far.
    pub directories_created: usize,

    /// The current operation being performed.
    pub current_operation: &'o DirectoryCopyOperation,

    /// The index of the current operation.
    ///
    /// Starts at `0`, goes up to (including) `total_operations - 1`.
    pub current_operation_index: usize,

    /// The total amount of operations that need to be performed to
    /// copy the requested directory.
    ///
    /// A single operation is either copying a file or creating a directory,
    /// see [`DirectoryCopyOperation`].
    pub total_operations: usize,
}

impl<'o> DirectoryCopyProgressRef<'o> {
    /// Clones the required data from this progress struct
    /// into an [`DirectoryCopyProgress`] - this way you own
    /// the entire state.
    pub fn to_owned_progress(&self) -> DirectoryCopyProgress {
        DirectoryCopyProgress {
            bytes_total: self.bytes_total,
            bytes_finished: self.bytes_finished,
            files_copied: self.files_copied,
            symlinks_created: self.symlinks_created,
            directories_created: self.directories_created,
            current_operation: self.current_operation.to_owned(),
            current_operation_index: self.current_operation_index,
            total_operations: self.total_operations,
        }
    }
}


/// Directory copying progress.
///
/// This is a fully-owned version of [`DirectoryCopyProgress`],
/// where the `current_operation` field is borrowed.
///
/// Obtainable from a reference to [`DirectoryCopyProgressRef`]
/// by calling [`DirectoryCopyProgressRef::to_owned_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryCopyProgress {
    /// Total number of bytes that need to be copied
    /// for the directory copy to be complete.
    pub bytes_total: u64,

    /// Number of bytes that have been copied so far.
    pub bytes_finished: u64,

    /// Number of files that have been copied so far.
    pub files_copied: usize,

    /// Number of symlinks that have been (re)created so far.
    ///
    /// If the [`DirectoryCopyOptions::symlink_behaviour`] option is set to
    /// [`SymlinkBehaviour::Follow`], this will always be `0`.
    pub symlinks_created: usize,

    /// Number of directories that have been created so far.
    pub directories_created: usize,

    /// The current operation being performed.
    pub current_operation: DirectoryCopyOperation,

    /// The index of the current operation.
    ///
    /// Starts at `0`, goes up to (including) `total_operations - 1`.
    pub current_operation_index: usize,

    /// The total amount of operations that need to be performed to
    /// copy the requested directory.
    ///
    /// A single operation is either copying a file or creating a directory,
    /// see [`DirectoryCopyOperation`].
    pub total_operations: usize,
}



#[derive(Clone, PartialEq, Eq, Debug)]
struct DirectoryCopyInternalProgress {
    /// Total number of bytes that need to be copied
    /// for the directory copy to be complete.
    bytes_total: u64,

    /// Number of bytes that have been copied so far.
    bytes_finished: u64,

    /// Number of files that have been copied so far.
    files_copied: usize,

    /// Number of symlinks that have been (re)created so far.
    ///
    /// If the [`DirectoryCopyOptions::symlink_behaviour`] option is set to
    /// [`SymlinkBehaviour::Follow`], this will always be `0`.
    symlinks_created: usize,

    /// Number of directories that have been created so far.
    directories_created: usize,

    /// The current operation being performed.
    current_operation: Option<DirectoryCopyOperation>,

    /// The index of the current operation.
    ///
    /// Starts at `0`, goes up to (including) `total_operations - 1`.
    current_operation_index: Option<usize>,

    /// The total amount of operations that need to be performed to
    /// copy the requested directory.
    ///
    /// A single operation is either copying a file or creating a directory,
    /// see [`DirectoryCopyOperation`].
    total_operations: usize,
}

impl DirectoryCopyInternalProgress {
    /// Modifies `self` with the provided `FnMut` closure.
    /// Then, the provided progress handler closure is called.
    fn update_operation_and_emit_progress<M, F>(
        &mut self,
        mut self_modifier_closure: M,
        progress_handler: &mut F,
    ) where
        M: FnMut(&mut Self),
        F: FnMut(&DirectoryCopyProgressRef),
    {
        self_modifier_closure(self);
        progress_handler(&self.to_user_facing_progress());
    }

    /// Replaces the current [`current_operation`][Self::current_operation]
    /// with the next one.
    ///
    /// The [`current_operation_index`][Self::current_operation_index]
    /// is incremented or set to 0, if previously unset.
    ///
    /// Finally, the provided progress handler closure is called.
    fn set_next_operation_and_emit_progress<F>(
        &mut self,
        operation: DirectoryCopyOperation,
        progress_handler: &mut F,
    ) where
        F: FnMut(&DirectoryCopyProgressRef),
    {
        if let Some(existing_operation_index) = self.current_operation_index.as_mut() {
            *existing_operation_index += 1;
        } else {
            self.current_operation_index = Some(0);
        }

        self.current_operation = Some(operation);

        progress_handler(&self.to_user_facing_progress())
    }

    /// Converts the [`DirectoryCopyInternalProgress`] to a [`DirectoryCopyProgress`],
    /// copying only the small fields, and passing the `current_operation` as a reference.
    ///
    /// # Panics
    /// Panics if the `current_operation` or `current_operation_index` field is `None`.
    fn to_user_facing_progress(&self) -> DirectoryCopyProgressRef<'_> {
        let current_operation_reference = self
            .current_operation
            .as_ref()
            // PANIC SATEFY: The caller is responsible.
            .expect("current_operation field to be Some");

        let current_operation_index = self
            .current_operation_index
            // PANIC SATEFY: The caller is responsible.
            .expect("current_operation_index to be Some");

        DirectoryCopyProgressRef {
            bytes_total: self.bytes_total,
            bytes_finished: self.bytes_finished,
            files_copied: self.files_copied,
            symlinks_created: self.symlinks_created,
            directories_created: self.directories_created,
            current_operation: current_operation_reference,
            current_operation_index,
            total_operations: self.total_operations,
        }
    }
}




/// Options that influence the [`copy_directory_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyWithProgressOptions {
    /// Specifies whether you allow the destination directory to exist before copying,
    /// and whether you require it to be empty. If you allow a non-empty destination directory,
    /// you may also specify how to handle existing destination files and sub-directories.
    ///
    /// See [`DestinationDirectoryRule`] documentation for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Maximum depth of the source directory to copy.
    pub copy_depth_limit: DirectoryCopyDepthLimit,

    /// Sets the behaviour for symbolic links when copying a directory.
    pub symlink_behaviour: SymlinkBehaviour,

    /// Sets the behaviour for broken symbolic links when copying a directory.
    pub broken_symlink_behaviour: BrokenSymlinkBehaviour,

    /// Internal buffer size used for reading from source files.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to destination files.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    ///
    /// Defaults to 512 KiB.
    ///
    /// *Note that the real reporting interval can be larger*
    /// (see [`copy_directory_with_progress`] for more info).
    ///
    ///
    /// [`copy_directory_with_progress`]: copy_directory_with_progress#progress-reporting
    pub progress_update_byte_interval: u64,
}

impl Default for DirectoryCopyWithProgressOptions {
    /// Constructs defaults for copying a directory, which are:
    /// - [`DestinationDirectoryRule::AllowEmpty`]: if the destination directory already exists, it must be empty,
    /// - [`DirectoryCopyDepthLimit::Unlimited`]: there is no copy depth limit,
    /// - [`SymlinkBehaviour::Keep`]: symbolic links are not followed,
    /// - [`BrokenSymlinkBehaviour::Keep`]: broken symbolic links are kept as-is, i.e. broken,
    /// - the read and write buffers are 64 KiB large, and
    /// - the progress reporting closure byte interval is set to 512 KiB.
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Abort,
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            write_buffer_size: DEFAULT_WRITE_BUFFER_SIZE,
            progress_update_byte_interval: DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL,
        }
    }
}



/// Given inner data of [`QueuedOperation::CopyFile`], this function
/// copies the given file, with progress information.
///
/// The function respects given `options`.
fn execute_copy_file_operation_with_progress<F>(
    source_file_path: PathBuf,
    source_size_bytes: u64,
    destination_path: PathBuf,
    options: &DirectoryCopyWithProgressOptions,
    progress: &mut DirectoryCopyInternalProgress,
    progress_handler: &mut F,
) -> Result<(), CopyDirectoryExecutionError>
where
    F: FnMut(&DirectoryCopyProgressRef),
{
    let can_overwrite_destination_file = options
        .destination_directory_rule
        .allows_overwriting_existing_destination_files();



    let destination_path_exists =
        try_exists_without_follow(&destination_path).map_err(|error| {
            CopyDirectoryExecutionError::UnableToAccessDestination {
                path: destination_path.clone(),
                error,
            }
        })?;

    if destination_path_exists {
        let destination_path_metadata =
            fs::symlink_metadata(&destination_path).map_err(|error| {
                CopyDirectoryExecutionError::UnableToAccessDestination {
                    path: destination_path.clone(),
                    error,
                }
            })?;


        if !destination_path_metadata.is_file() {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: destination_path.clone(),
            });
        }

        if !can_overwrite_destination_file {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: destination_path.clone(),
            });
        }
    }


    progress.set_next_operation_and_emit_progress(
        DirectoryCopyOperation::CopyingFile {
            destination_file_path: destination_path.clone(),
            progress: FileProgress {
                bytes_finished: 0,
                bytes_total: source_size_bytes,
            },
        },
        progress_handler,
    );


    // Set to `true` when we update our `bytes_total` to the
    // freshly calculated total number of bytes in a file (after the copying starts).
    let mut updated_bytes_total_with_fresh_value = false;

    // Number of `bytes_copied` last emitted through the progress closure.
    let bytes_copied_before = progress.bytes_finished;


    copy_file_with_progress(
        source_file_path,
        &destination_path,
        FileCopyWithProgressOptions {
            colliding_file_behaviour: match options.destination_directory_rule {
                DestinationDirectoryRule::DisallowExisting => CollidingFileBehaviour::Abort,
                DestinationDirectoryRule::AllowEmpty => CollidingFileBehaviour::Abort,
                DestinationDirectoryRule::AllowNonEmpty { colliding_file_behaviour, .. } => colliding_file_behaviour,
            },
            read_buffer_size: options.read_buffer_size,
            write_buffer_size: options.write_buffer_size,
            progress_update_byte_interval: options.progress_update_byte_interval,
        },
        |new_file_progress| progress.update_operation_and_emit_progress(
                |progress| {
                    let current_operation = progress.current_operation.as_mut()
                        // PANIC SATEFY: The function calls `set_next_operation_and_emit_progress` above,
                        // meaning the `current_operation` can never be None.
                        .expect("the current_operation field to be Some");


                    if let DirectoryCopyOperation::CopyingFile {
                        progress: file_progress,
                        ..
                    } = current_operation
                    {
                        // It is somewhat possible that a file is written to between the scanning phase 
                        // and copying. In that case, it is *possible* that the file size changes, 
                        // which means we should listen to the size `copy_file_with_progress` 
                        // is reporting. There is no point to doing this each update, so we do it only once.
                        if !updated_bytes_total_with_fresh_value {
                            file_progress.bytes_total = new_file_progress.bytes_total;
                            updated_bytes_total_with_fresh_value = true;
                        }

                        file_progress.bytes_finished = new_file_progress.bytes_finished;
                        progress.bytes_finished =
                            bytes_copied_before + file_progress.bytes_finished;
                    } else {
                        // PANIC SAFETY: Since we set `progress` to a `CopyingFile` 
                        // at the beginning of the function, and there is no possibility 
                        // of changing that operation in between, this panic should never happen.
                        panic!(
                            "BUG: `progress.current_operation` doesn't match DirectoryCopyOperation::CopyingFile"
                        );
                    }
                },
                progress_handler,
            )
    )
        .map_err(|file_error| CopyDirectoryExecutionError::FileCopyError { file_path: destination_path, error: file_error })?;


    progress.files_copied += 1;

    Ok(())
}


/// Given inner data of [`QueuedOperation::CreateDirectory`], this function
/// creates the given directory with progress information.
///
/// If the directory already exists, no action
/// is taken, unless the given options indicate that to be an error
/// (`overwrite_existing_subdirectories`, see [`DestinationDirectoryRule`]).
///
/// If the given path exists, but is not a directory, an error is returned as well.
fn execute_create_directory_operation_with_progress<F>(
    destination_directory_path: PathBuf,
    source_size_bytes: u64,
    options: &DirectoryCopyWithProgressOptions,
    progress: &mut DirectoryCopyInternalProgress,
    progress_handler: &mut F,
) -> Result<(), CopyDirectoryExecutionError>
where
    F: FnMut(&DirectoryCopyProgressRef),
{
    let destination_directory_exists = try_exists_without_follow(&destination_directory_path)
        .map_err(|error| CopyDirectoryExecutionError::UnableToAccessDestination {
            path: destination_directory_path.clone(),
            error,
        })?;

    if destination_directory_exists {
        let destination_directory_metadata = fs::symlink_metadata(&destination_directory_path)
            .map_err(|error| CopyDirectoryExecutionError::UnableToAccessDestination {
                path: destination_directory_path.clone(),
                error,
            })?;

        if !destination_directory_metadata.is_dir() {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: destination_directory_path,
            });
        }

        if options.destination_directory_rule == DestinationDirectoryRule::DisallowExisting {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: destination_directory_path,
            });
        }

        // If the destination directory rule does not forbid an existing sub-directory,
        // we have no directory to create, since it already exists.
        return Ok(());
    }


    progress.set_next_operation_and_emit_progress(
        DirectoryCopyOperation::CreatingDirectory {
            destination_directory_path: destination_directory_path.clone(),
        },
        progress_handler,
    );

    fs::create_dir(&destination_directory_path).map_err(|error| {
        CopyDirectoryExecutionError::UnableToCreateDirectory {
            directory_path: destination_directory_path,
            error,
        }
    })?;


    progress.directories_created += 1;
    progress.bytes_finished += source_size_bytes;

    Ok(())
}



struct SymlinkCreationInfo {
    symlink_path: PathBuf,

    symlink_destination_path: PathBuf,

    #[cfg(windows)]
    symlink_type: crate::directory::prepared::SymlinkType,

    unfollowed_symlink_file_size_bytes: u64,
}


fn execute_create_symlink_operation_with_progress<F>(
    symlink_info: SymlinkCreationInfo,
    options: &DirectoryCopyWithProgressOptions,
    progress: &mut DirectoryCopyInternalProgress,
    progress_handler: &mut F,
) -> Result<(), CopyDirectoryExecutionError>
where
    F: FnMut(&DirectoryCopyProgressRef),
{
    let can_overwrite_destination_file = options
        .destination_directory_rule
        .allows_overwriting_existing_destination_files();


    let symlink_path_exists =
        try_exists_without_follow(&symlink_info.symlink_path).map_err(|error| {
            CopyDirectoryExecutionError::UnableToAccessDestination {
                path: symlink_info.symlink_path.clone(),
                error,
            }
        })?;

    if symlink_path_exists {
        let symlink_path_metadata =
            fs::symlink_metadata(&symlink_info.symlink_path).map_err(|error| {
                CopyDirectoryExecutionError::UnableToAccessDestination {
                    path: symlink_info.symlink_path.clone(),
                    error,
                }
            })?;

        if !symlink_path_metadata.is_symlink() {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: symlink_info.symlink_path,
            });
        }

        if !can_overwrite_destination_file {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: symlink_info.symlink_path,
            });
        }
    }


    progress.set_next_operation_and_emit_progress(
        DirectoryCopyOperation::CreatingSymbolicLink {
            destination_symbolic_link_file_path: symlink_info.symlink_path.clone(),
        },
        progress_handler,
    );


    #[cfg(windows)]
    {
        use crate::directory::prepared::SymlinkType;

        match symlink_info.symlink_type {
            SymlinkType::File => {
                std::os::windows::fs::symlink_file(
                    &symlink_info.symlink_destination_path,
                    &symlink_info.symlink_path,
                )
                .map_err(|error| {
                    CopyDirectoryExecutionError::SymlinkCreationError {
                        symlink_path: symlink_info.symlink_path.clone(),
                        error,
                    }
                })?;
            }
            SymlinkType::Directory => {
                std::os::windows::fs::symlink_dir(
                    &symlink_info.symlink_destination_path,
                    &symlink_info.symlink_path,
                )
                .map_err(|error| {
                    CopyDirectoryExecutionError::SymlinkCreationError {
                        symlink_path: symlink_info.symlink_path.clone(),
                        error,
                    }
                })?;
            }
        };
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            &symlink_info.symlink_destination_path,
            &symlink_info.symlink_path,
        )
        .map_err(|error| CopyDirectoryExecutionError::SymlinkCreationError {
            symlink_path: symlink_info.symlink_path.clone(),
            error,
        })?;
    }


    progress.symlinks_created += 1;
    progress.bytes_finished += symlink_info.unfollowed_symlink_file_size_bytes;

    Ok(())
}




/// Execute a prepared copy with progress tracking.
///
/// For more details, see [`copy_directory_with_progress`].
pub(crate) fn execute_prepared_copy_directory_with_progress_unchecked<F>(
    prepared_copy: DirectoryCopyPrepared,
    options: DirectoryCopyWithProgressOptions,
    mut progress_handler: F,
) -> Result<DirectoryCopyFinished, CopyDirectoryExecutionError>
where
    F: FnMut(&DirectoryCopyProgressRef),
{
    let validated_destination = prepared_copy.validated_destination_directory;

    // Create destination directory if needed.
    let mut progress = if validated_destination.state.exists() {
        if options.destination_directory_rule == DestinationDirectoryRule::DisallowExisting {
            return Err(CopyDirectoryExecutionError::DestinationEntryUnexpected {
                path: validated_destination.directory_path,
            });
        }

        DirectoryCopyInternalProgress {
            bytes_total: prepared_copy.total_bytes,
            bytes_finished: 0,
            files_copied: 0,
            symlinks_created: 0,
            directories_created: 0,
            // This is an invisible operation - we don't emit this progress struct at all,
            // but we do need something here before the next operation starts.
            current_operation: None,
            current_operation_index: None,
            total_operations: prepared_copy.operation_queue.len(),
        }
    } else {
        // This time we actually emit progress after creating the destination directory.

        let mut progress = DirectoryCopyInternalProgress {
            bytes_total: prepared_copy.total_bytes,
            bytes_finished: 0,
            files_copied: 0,
            symlinks_created: 0,
            directories_created: 0,
            current_operation: Some(DirectoryCopyOperation::CreatingDirectory {
                destination_directory_path: validated_destination.directory_path.clone(),
            }),
            current_operation_index: Some(0),
            total_operations: prepared_copy.operation_queue.len() + 1,
        };

        progress_handler(&progress.to_user_facing_progress());

        fs::create_dir_all(&validated_destination.directory_path).map_err(|error| {
            CopyDirectoryExecutionError::UnableToCreateDirectory {
                directory_path: validated_destination.directory_path.clone(),
                error,
            }
        })?;

        progress.directories_created += 1;

        progress
    };


    // Execute queued directory copy operations.
    for operation in prepared_copy.operation_queue {
        match operation {
            QueuedOperation::CopyFile {
                source_file_path: source_path,
                source_size_bytes,
                destination_file_path,
            } => execute_copy_file_operation_with_progress(
                source_path,
                source_size_bytes,
                destination_file_path,
                &options,
                &mut progress,
                &mut progress_handler,
            )?,

            QueuedOperation::CreateDirectory {
                source_size_bytes,
                destination_directory_path,
            } => execute_create_directory_operation_with_progress(
                destination_directory_path,
                source_size_bytes,
                &options,
                &mut progress,
                &mut progress_handler,
            )?,

            #[cfg(windows)]
            QueuedOperation::CreateSymlink {
                symlink_path,
                symlink_type,
                source_symlink_size_bytes,
                symlink_destination_path,
            } => execute_create_symlink_operation_with_progress(
                SymlinkCreationInfo {
                    symlink_path,
                    symlink_destination_path,
                    symlink_type,
                    unfollowed_symlink_file_size_bytes: source_symlink_size_bytes,
                },
                &options,
                &mut progress,
                &mut progress_handler,
            )?,

            #[cfg(unix)]
            QueuedOperation::CreateSymlink {
                symlink_path,
                source_symlink_size_bytes,
                symlink_destination_path,
            } => execute_create_symlink_operation_with_progress(
                SymlinkCreationInfo {
                    symlink_path,
                    symlink_destination_path,
                    unfollowed_symlink_file_size_bytes: source_symlink_size_bytes,
                },
                &options,
                &mut progress,
                &mut progress_handler,
            )?,
        }
    }

    // One last progress update - everything should be done at this point.
    progress_handler(&progress.to_user_facing_progress());

    Ok(DirectoryCopyFinished {
        total_bytes_copied: progress.bytes_finished,
        files_copied: progress.files_copied,
        symlinks_created: progress.symlinks_created,
        directories_created: progress.directories_created,
    })
}


/// Copies a directory from the source to the destination directory, with progress reporting.
///
/// Contents of the source directory will be copied into the destination directory.
/// If needed, the destination directory will be created before copying begins.
///
///
/// # Symbolic links
/// If the provided `source_directory_path` is itself a symlink that points to a directory,
/// the link will be followed and the contents of the link target directory will be copied.
///
/// Regarding symbolic links *inside* the source directory, the chosen [`symlink_behaviour`] is respected.
///
/// This matches the behaviour of `cp` with `--recursive` (and optionally `--dereference`)
/// flags on Unix[^unix-cp-rd].
///
///
/// # Options
/// See [`DirectoryCopyWithProgressOptions`] for the full set of available directory copying options.
///
/// ### `destination_directory_rule` considerations
/// If you allow the destination directory to exist and be non-empty,
/// source directory contents will be merged (!) into the destination directory.
/// This is *not* the default, and you should probably consider the consequences
/// very carefully before setting the corresponding [`options.destination_directory_rule`]
/// option to anything other than [`DisallowExisting`] or [`AllowEmpty`].
///
///
/// # Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`DirectoryCopyFinished`].
///
///
/// ## Progress reporting
/// This function allows you to receive progress reports by passing
/// a `progress_handler` closure. It will be called with
/// a reference to [`DirectoryCopyProgress`] regularly.
///
/// You can control the progress reporting frequency by setting the
/// [`options.progress_update_byte_interval`] option to a sufficiently small or large value,
/// but note that smaller intervals are likely to have an additional impact on performance.
/// The value of this option is the minimum amount of bytes written to a file between
/// two calls to the provided `progress_handler`.
///
/// This function does not guarantee a precise number of progress reports;
/// it does, however, guarantee at least one progress report per file copy, symlink and directory creation operation.
/// It also guarantees one final progress report, when the state indicates the copy has been completed.
///
/// For more details on reporting intervals for file copies, see progress reporting section
/// for [`copy_file`][crate::file::copy_file].
///
///
/// # Errors
/// If the directory cannot be copied to the destination, a [`CopyDirectoryError`] is returned;
/// see its documentation for more details.
///
/// Errors for this function are quite granular, and are split into two main groups:
/// - Preparation errors ([`CopyDirectoryError::PreparationError`]) are emitted during
///   the preparation phase of copying. Importantly, if an error from this group is returned,
///   the destination directory *hasn't been changed yet* in any way.
/// - Copy execution errors ([`CopyDirectoryError::ExecutionError`]) are emitted during
///   the actual copying phase. If an error from this group is returned,
///   it is very likely that the destination directory is in an unpredictable state, since
///   the error occurred while trying to copy a file or create a directory.
///
///
/// [`options.progress_update_byte_interval`]: DirectoryCopyWithProgressOptions::progress_update_byte_interval
/// [`options.destination_directory_rule`]: DirectoryCopyWithProgressOptions::destination_directory_rule
/// [`options.copy_depth_limit`]: DirectoryCopyWithProgressOptions::copy_depth_limit
/// [`symlink_behaviour`]: DirectoryCopyWithProgressOptions::symlink_behaviour
/// [`DisallowExisting`]: DestinationDirectoryRule::DisallowExisting
/// [`AllowEmpty`]: DestinationDirectoryRule::AllowEmpty
/// [`AllowNonEmpty`]: DestinationDirectoryRule::AllowNonEmpty
/// [`copy_file`]: crate::file::copy_file
/// [^unix-cp-rd]: Source for coreutils' `cp` is available
///     [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/cp.c).
pub fn copy_directory_with_progress<S, T, F>(
    source_directory_path: S,
    destination_directory_path: T,
    options: DirectoryCopyWithProgressOptions,
    progress_handler: F,
) -> Result<DirectoryCopyFinished, CopyDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryCopyProgressRef),
{
    let prepared_copy = DirectoryCopyPrepared::prepare(
        source_directory_path.as_ref(),
        destination_directory_path.as_ref(),
        options.destination_directory_rule,
        options.copy_depth_limit,
        options.symlink_behaviour,
        options.broken_symlink_behaviour,
    )?;


    let finished_copy = execute_prepared_copy_directory_with_progress_unchecked(
        prepared_copy,
        options,
        progress_handler,
    )?;

    Ok(finished_copy)
}
