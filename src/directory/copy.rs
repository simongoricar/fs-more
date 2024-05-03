use std::path::{Path, PathBuf};

use_enabled_fs_module!();

use super::{
    common::DestinationDirectoryRule,
    prepared::{DirectoryCopyPrepared, QueuedOperation},
};
use crate::{
    error::{CopyDirectoryError, CopyDirectoryExcutionError},
    file::{
        copy_file,
        copy_file_with_progress,
        CopyFileOptions,
        CopyFileWithProgressOptions,
        ExistingFileBehaviour,
        FileProgress,
    },
    use_enabled_fs_module,
};

/// The maximum directory copy depth option.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DirectoryCopyDepthLimit {
    /// No depth limit.
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



/// Options for the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyDirectoryOptions {
    /// Specifies whether you allow the target directory to exist before copying
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Maximum depth of the source directory to copy over to the destination.
    pub copy_depth_limit: DirectoryCopyDepthLimit,
}

impl Default for CopyDirectoryOptions {
    /// Constructs defaults for copying a directory:
    /// - destination files aren't allowed to be overwritten (see [`DestinationDirectoryRule::AllowEmpty`]), and
    /// - there is no copy depth limit (see [`DirectoryCopyDepthLimit::Unlimited`]).
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
        }
    }
}


/// Describes actions taken by the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyDirectoryFinished {
    /// Total amount of bytes copied.
    pub total_bytes_copied: u64,

    /// Total number of files copied.
    pub files_copied: usize,

    /// Total number of directories created.
    pub directories_created: usize,
}



/// Perform a copy using prepared data from [`PreparedDirectoryCopy`].
///
/// This `*_unchecked` function does not validate the source and target paths;
/// it expects them to be already validated.
///
/// For more details, see [`copy_directory`].
pub(crate) fn copy_directory_unchecked(
    prepared_directory_copy: DirectoryCopyPrepared,
    options: CopyDirectoryOptions,
) -> Result<CopyDirectoryFinished, CopyDirectoryExcutionError> {
    let can_overwrite_files = options
        .destination_directory_rule
        .allows_overwriting_existing_destination_files();

    let can_ignore_existing_sub_directories = options
        .destination_directory_rule
        .ignores_existing_destination_sub_directories();


    // We have the entire queue of operations, and we've made sure there are
    // no collisions we should worry about. What's left is performing the file copy
    // and directory creation operations *precisely in the order they have been prepared*.
    // If we ignore the order, we could get into situations where
    // some destination directory doesn't exist yet, but we would want to copy a file into it.


    let mut total_bytes_copied = 0;
    let mut num_files_copied = 0;
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
        .map_err(
            |error| CopyDirectoryExcutionError::UnableToCreateDirectory {
                directory_path: prepared_directory_copy
                    .validated_destination_directory
                    .directory_path,
                error,
            },
        )?;

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
                let destination_file_exists =
                    destination_file_path.try_exists().map_err(|error| {
                        CopyDirectoryExcutionError::UnableToAccessDestination {
                            path: destination_file_path.clone(),
                            error,
                        }
                    })?;

                if destination_file_exists {
                    let destination_file_metadata = fs::symlink_metadata(&destination_file_path)
                        .map_err(
                            |error| CopyDirectoryExcutionError::UnableToAccessDestination {
                                path: destination_file_path.clone(),
                                error,
                            },
                        )?;


                    if !destination_file_metadata.is_file() {
                        return Err(
                            CopyDirectoryExcutionError::DestinationEntryUnexpected {
                                path: destination_file_path.clone(),
                            },
                        );
                    }

                    if !can_overwrite_files {
                        return Err(
                            CopyDirectoryExcutionError::DestinationEntryUnexpected {
                                path: destination_file_path.clone(),
                            },
                        );
                    }
                }


                copy_file(
                    source_file_path,
                    &destination_file_path,
                    CopyFileOptions {
                        existing_destination_file_behaviour: match can_overwrite_files {
                            true => ExistingFileBehaviour::Overwrite,
                            false => ExistingFileBehaviour::Abort,
                        },
                    },
                )
                .map_err(|file_error| {
                    CopyDirectoryExcutionError::FileCopyError {
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
                if destination_directory_path.exists() {
                    if !destination_directory_path.is_dir() {
                        return Err(
                            CopyDirectoryExcutionError::DestinationEntryUnexpected {
                                path: destination_directory_path.clone(),
                            },
                        );
                    }

                    if !can_ignore_existing_sub_directories {
                        return Err(
                            CopyDirectoryExcutionError::DestinationEntryUnexpected {
                                path: destination_directory_path.clone(),
                            },
                        );
                    }

                    continue;
                }

                fs::create_dir(&destination_directory_path).map_err(|error| {
                    CopyDirectoryExcutionError::UnableToCreateDirectory {
                        directory_path: destination_directory_path,
                        error,
                    }
                })?;

                num_directories_created += 1;
                total_bytes_copied += source_size_bytes;
            }
        };
    }

    Ok(CopyDirectoryFinished {
        total_bytes_copied,
        files_copied: num_files_copied,
        directories_created: num_directories_created,
    })
}


/// Copy a directory from `source_directory_path` to `destination_directory_path`.
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `destination_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `destination_directory_path` will be created.
///
///
/// ### Target directory
/// Depending on the [`options.destination_directory_rule`][DirectoryCopyOptions::destination_directory_rule] option,
/// the `destination_directory_path` must:
/// - with [`DisallowExisting`][DestinationDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][DestinationDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][DestinationDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see fields).
///
/// If the specified destination directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidDestinationDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::DestinationDirectoryNotEmpty`]`)` is returned (depending on the rule).
///
///
/// ### Copy depth
/// Depending on the [`DirectoryCopyOptions::maximum_copy_depth`] option, calling this function means copying:
/// - `Some(0)` -- a single directory and its direct descendants (files and direct directories, but *not their contents*, i.e. just empty directories),
/// - `Some(>=1)` -- files and subdirectories (and their files and directories, etc.) up to a certain depth limit (e.g. `Some(1)` copies direct descendants as well as one layer deeper),
/// - `None` -- the entire subtree. **This is probably what you want most of the time**.
///
///
/// ## Symbolic links
/// - If the `source_directory_path` directory contains a symbolic link to a file,
/// the contents of the file it points to will be copied
/// into the corresponding subpath inside `destination_directory_path`
/// (same behaviour as `cp` without `-P` on Unix, i.e. link is followed, but not preserved).
/// - If the `source_directory_path` directory contains a symbolic link to a directory,
/// the directory and its contents will be copied as normal - the links will be followed, but not preserved.
///
///
/// ### Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`FinishedDirectoryCopy`].
pub fn copy_directory<S, T>(
    source_directory_path: S,
    destination_directory_path: T,
    options: CopyDirectoryOptions,
) -> Result<CopyDirectoryFinished, CopyDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let prepared_copy = DirectoryCopyPrepared::prepare(
        source_directory_path.as_ref(),
        destination_directory_path.as_ref(),
        options.destination_directory_rule,
        options.copy_depth_limit,
    )?;

    let finished_copy = copy_directory_unchecked(prepared_copy, options)?;

    Ok(finished_copy)
}


/// Describes a directory copy operation.
///
/// Used in progress reporting in [`copy_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CopyDirectoryOperation {
    /// A directory is being created.
    CreatingDirectory {
        /// Path of the directory that is being created.
        destination_directory_path: PathBuf,
    },

    /// A file is being copied.
    ///
    /// For more precise copying progress, see the `progress` field.
    CopyingFile {
        /// Path of the file is being created.
        destination_file_path: PathBuf,

        /// Progress of the file copy operation.
        progress: FileProgress,
    },
}


/// Represents the progress of copying a directory.
///
/// Used to report directory copying progress to a user-provided closure, see [`copy_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CopyDirectoryProgress {
    /// Amount of bytes that need to be copied for the directory copy to be complete.
    pub bytes_total: u64,

    /// Amount of bytes that have been copied so far.
    pub bytes_finished: u64,

    /// Number of files that have been copied so far.
    pub files_copied: usize,

    /// Number of directories that have been created so far.
    pub directories_created: usize,

    /// The current operation being performed.
    pub current_operation: CopyDirectoryOperation,

    /// The index of the current operation (starts at `0`, goes up to (including) `total_operations - 1`).
    pub current_operation_index: isize,

    /// The total amount of operations that need to be performed to copy the requested directory.
    ///
    /// A single operation is either copying a file or creating a directory, see [`CopyDirectoryOperation`].
    pub total_operations: isize,
}

impl CopyDirectoryProgress {
    /// Update the current [`CopyDirectoryOperation`] with the given closure.
    /// After updating the operation, this function calls the given progress handler.
    fn update_operation_and_emit_progress<M, F>(
        &mut self,
        mut modifer_closure: M,
        progress_handler: &mut F,
    ) where
        M: FnMut(&mut Self),
        F: FnMut(&CopyDirectoryProgress),
    {
        modifer_closure(self);

        progress_handler(self);
    }

    /// Replace the current [`CopyDirectoryOperation`] with the next one (incrementing the operation index).
    /// After updating the operation, this function calls the given progress handler.
    fn set_next_operation_and_emit_progress<F>(
        &mut self,
        operation: CopyDirectoryOperation,
        progress_handler: &mut F,
    ) where
        F: FnMut(&CopyDirectoryProgress),
    {
        self.current_operation_index += 1;
        self.current_operation = operation;

        progress_handler(self)
    }
}



/// Options that influence the [`copy_directory_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyDirectoryWithProgressOptions {
    /// Specifies whether you allow the destination directory to exist before copying and whether it must be empty or not.
    /// If you allow a non-empty destination directory, you may also specify whether you allow
    /// destination files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`DestinationDirectoryRule`] for more details and examples.
    pub destination_directory_rule: DestinationDirectoryRule,

    /// Maximum depth of the source directory to copy.
    pub copy_depth_limit: DirectoryCopyDepthLimit,

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

impl Default for CopyDirectoryWithProgressOptions {
    fn default() -> Self {
        Self {
            destination_directory_rule: DestinationDirectoryRule::default(),
            copy_depth_limit: DirectoryCopyDepthLimit::Unlimited,
            // 64 KiB
            read_buffer_size: 1024 * 64,
            // 64 KiB
            write_buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}



/// Given inner data of [`QueuedOperation::CopyFile`], this function
/// copies the given file with progress information.
///
/// The function respects given `options` (e.g. returning an error
/// if the file already exists if configured to do so).
fn execute_copy_file_operation_with_progress<F>(
    source_file_path: PathBuf,
    source_size_bytes: u64,
    destination_path: PathBuf,
    options: &CopyDirectoryWithProgressOptions,
    progress: &mut CopyDirectoryProgress,
    progress_handler: &mut F,
) -> Result<(), CopyDirectoryExcutionError>
where
    F: FnMut(&CopyDirectoryProgress),
{
    let can_overwrite_destination_file = options
        .destination_directory_rule
        .allows_overwriting_existing_destination_files();



    let destination_path_exists = destination_path.try_exists().map_err(|error| {
        CopyDirectoryExcutionError::UnableToAccessDestination {
            path: destination_path.clone(),
            error,
        }
    })?;

    if destination_path_exists {
        let destination_path_metadata =
            fs::symlink_metadata(&destination_path).map_err(|error| {
                CopyDirectoryExcutionError::UnableToAccessDestination {
                    path: destination_path.clone(),
                    error,
                }
            })?;


        if !destination_path_metadata.is_file() {
            return Err(
                CopyDirectoryExcutionError::DestinationEntryUnexpected {
                    path: destination_path.clone(),
                },
            );
        }

        if !can_overwrite_destination_file {
            return Err(
                CopyDirectoryExcutionError::DestinationEntryUnexpected {
                    path: destination_path.clone(),
                },
            );
        }
    }


    progress.set_next_operation_and_emit_progress(
        CopyDirectoryOperation::CopyingFile {
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
        CopyFileWithProgressOptions {
            existing_destination_file_behaviour: match options.destination_directory_rule {
                DestinationDirectoryRule::DisallowExisting => ExistingFileBehaviour::Abort,
                DestinationDirectoryRule::AllowEmpty => ExistingFileBehaviour::Abort,
                DestinationDirectoryRule::AllowNonEmpty { existing_destination_file_behaviour, .. } => existing_destination_file_behaviour,
            },
            read_buffer_size: options.read_buffer_size,
            write_buffer_size: options.write_buffer_size,
            progress_update_byte_interval: options.progress_update_byte_interval,
        },
        |new_file_progress| progress.update_operation_and_emit_progress(
                |progress| {
                    if let CopyDirectoryOperation::CopyingFile {
                        progress: file_progress,
                        ..
                    } = &mut progress.current_operation
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
                            "BUG: `progress.current_operation` doesn't match CopyDirectoryOperation::CopyingFile"
                        );
                    }
                },
                progress_handler,
            )
    )
        .map_err(|file_error| CopyDirectoryExcutionError::FileCopyError { file_path: destination_path, error: file_error })?;


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
    options: &CopyDirectoryWithProgressOptions,
    progress: &mut CopyDirectoryProgress,
    progress_handler: &mut F,
) -> Result<(), CopyDirectoryExcutionError>
where
    F: FnMut(&CopyDirectoryProgress),
{
    let destination_directory_exists =
        destination_directory_path.try_exists().map_err(|error| {
            CopyDirectoryExcutionError::UnableToAccessDestination {
                path: destination_directory_path.clone(),
                error,
            }
        })?;

    if destination_directory_exists {
        let destination_directory_metadata = fs::symlink_metadata(&destination_directory_path)
            .map_err(
                |error| CopyDirectoryExcutionError::UnableToAccessDestination {
                    path: destination_directory_path.clone(),
                    error,
                },
            )?;

        if !destination_directory_metadata.is_dir() {
            return Err(
                CopyDirectoryExcutionError::DestinationEntryUnexpected {
                    path: destination_directory_path,
                },
            );
        }

        if options.destination_directory_rule == DestinationDirectoryRule::DisallowExisting {
            return Err(
                CopyDirectoryExcutionError::DestinationEntryUnexpected {
                    path: destination_directory_path,
                },
            );
        }

        // If the destination directory rule does not forbid an existing sub-directory,
        // we have no directory to create, since it already exists.
        return Ok(());
    }


    progress.set_next_operation_and_emit_progress(
        CopyDirectoryOperation::CreatingDirectory {
            destination_directory_path: destination_directory_path.clone(),
        },
        progress_handler,
    );

    fs::create_dir(&destination_directory_path).map_err(|error| {
        CopyDirectoryExcutionError::UnableToCreateDirectory {
            directory_path: destination_directory_path,
            error,
        }
    })?;


    progress.directories_created += 1;
    progress.bytes_finished += source_size_bytes;

    Ok(())
}



/// Perform a with-progress copy from `source_directory_path` to `validated_target`.
///
/// This `*_unchecked` function does not validate the source and target paths (i.e. expects them to be already validated).
///
/// For more details, see [`copy_directory_with_progress`].
pub(crate) fn perform_prepared_copy_directory_with_progress_unchecked<F>(
    prepared_copy: DirectoryCopyPrepared,
    options: CopyDirectoryWithProgressOptions,
    mut progress_handler: F,
) -> Result<CopyDirectoryFinished, CopyDirectoryExcutionError>
where
    F: FnMut(&CopyDirectoryProgress),
{
    // let allows_existing_destination_directory = options
    //     .destination_directory_rule
    //     .allows_existing_destination_directory();
    // let should_overwrite_directories = options
    //     .destination_directory_rule
    //     .allows_creating_missing_directories();


    let validated_destination = prepared_copy.validated_destination_directory;

    // Create destination directory if needed.
    let mut progress = if validated_destination.state.exists() {
        if options.destination_directory_rule == DestinationDirectoryRule::DisallowExisting {
            return Err(
                CopyDirectoryExcutionError::DestinationEntryUnexpected {
                    path: validated_destination.directory_path,
                },
            );
        }

        CopyDirectoryProgress {
            bytes_total: prepared_copy.total_bytes,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            // This is an invisible operation - we don't emit this progress struct at all,
            // but we do need something here before the next operation starts.
            current_operation: CopyDirectoryOperation::CreatingDirectory {
                destination_directory_path: PathBuf::new(),
            },
            current_operation_index: -1,
            total_operations: prepared_copy.operation_queue.len() as isize,
        }
    } else {
        // This time we actually emit progress after creating the destination directory.

        let mut progress = CopyDirectoryProgress {
            bytes_total: prepared_copy.total_bytes,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            current_operation: CopyDirectoryOperation::CreatingDirectory {
                destination_directory_path: validated_destination.directory_path.clone(),
            },
            current_operation_index: 0,
            total_operations: prepared_copy.operation_queue.len() as isize + 1,
        };

        progress_handler(&progress);

        fs::create_dir_all(&validated_destination.directory_path).map_err(|error| {
            CopyDirectoryExcutionError::UnableToCreateDirectory {
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
        }
    }

    // One last progress update - everything should be done at this point.
    progress_handler(&progress);

    Ok(CopyDirectoryFinished {
        total_bytes_copied: progress.bytes_finished,
        files_copied: progress.files_copied,
        directories_created: progress.directories_created,
    })
}


/// Copy an entire directory from `source_directory_path` to `destination_directory_path`
/// (with progress reporting).
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `destination_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `destination_directory_path` will be created.
///
///
/// ### Target directory
/// Depending on the [`options.destination_directory_rule`][DirectoryCopyOptions::destination_directory_rule] option,
/// the `destination_directory_path` must:
/// - with [`DisallowExisting`][DestinationDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][DestinationDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][DestinationDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see variant's fields).
///
/// If the specified destination directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidDestinationDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::DestinationDirectoryNotEmpty`]`)` is returned (depending on the rule).
///
///
/// ## Progress reporting
/// You must also provide a progress handler closure that will receive
/// a [`&DirectoryCopyProgress`][DirectoryCopyProgress] containing progress state.
///
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`][DirectoryCopyWithProgressOptions::progress_update_byte_interval]
/// option. The value of that option is the *minimum* amount of bytes written to a single file
/// between two progress reports (defaults to 64 KiB).
/// As such, this function does not guarantee a fixed amount of progress reports per file size.
/// It does, however, guarantee *at least one progress report per file copy operation and per directory creation operation*.
/// It also guarantees one final progress report, when the state indicates copy completion.
///
///
/// ## Copy depth
/// TODO update documentation
/// Depending on the [`options.maximum_copy_depth`][DirectoryCopyWithProgressOptions::maximum_copy_depth]
/// option, calling this function means copying:
/// - `Some(0)` -- a single directory and its direct descendants (files and direct directories, but *not their contents*, i.e. just empty directories),
/// - `Some(>=1)` -- files and subdirectories (and their files and directories, etc.) up to a certain depth limit (e.g. `Some(1)` copies direct descendants as well as one layer deeper),
/// - `None` -- the entire subtree. **This is probably what you want most of the time**.
///
///
/// ## Symbolic links
/// - If the `source_directory_path` directory contains a symbolic link to a file,
/// the contents of the file it points to will be copied
/// into the corresponding subpath inside `target_file_path`
/// (same behaviour as `cp` without `-P` on Unix, i.e. link is followed, but not preserved).
/// - If the `source_directory_path` directory contains a symbolic link to a directory,
/// the directory and its contents will be copied as normal - the links will be followed, but not preserved.
///
///
/// ## Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`FinishedDirectoryCopy`].
pub fn copy_directory_with_progress<S, T, F>(
    source_directory_path: S,
    destination_directory_path: T,
    options: CopyDirectoryWithProgressOptions,
    progress_handler: F,
) -> Result<CopyDirectoryFinished, CopyDirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&CopyDirectoryProgress),
{
    let prepared_copy = DirectoryCopyPrepared::prepare(
        source_directory_path.as_ref(),
        destination_directory_path.as_ref(),
        options.destination_directory_rule,
        options.copy_depth_limit,
    )?;


    let finished_copy = perform_prepared_copy_directory_with_progress_unchecked(
        prepared_copy,
        options,
        progress_handler,
    )?;

    Ok(finished_copy)
}
