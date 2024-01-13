#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "fs-err")]
use fs_err as fs;

use super::{
    common::TargetDirectoryRule,
    prepared::{PreparedDirectoryCopy, QueuedOperation},
};
use crate::{
    error::{DirectoryError, FileError},
    file::{
        copy_file,
        copy_file_with_progress,
        FileCopyOptions,
        FileCopyWithProgressOptions,
        FileProgress,
    },
};



/// Options that influence the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyOptions {
    /// Specifies whether you allow the target directory to exist before copying
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// target files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`TargetDirectoryRule`] for more details and examples.
    pub target_directory_rule: TargetDirectoryRule,

    /// Maximum depth of the source directory to copy.
    ///
    /// - `None` indicates no limit.
    /// - `Some(0)` means a directory copy operation that copies only the files and
    ///   creates directories found directly in the root directory, ignoring any subdirectories.
    /// - `Some(1)` includes the root directory's contents and one level of its subdirectories.
    pub maximum_copy_depth: Option<usize>,
}

#[allow(clippy::derivable_impls)]
impl Default for DirectoryCopyOptions {
    fn default() -> Self {
        Self {
            target_directory_rule: TargetDirectoryRule::default(),
            maximum_copy_depth: None,
        }
    }
}


/// Describes actions taken by the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FinishedDirectoryCopy {
    /// Total amount of bytes copied.
    pub total_bytes_copied: u64,

    /// Number of files copied when copying the directory.
    pub num_files_copied: usize,

    /// Number of directories created when copying the directory.
    pub num_directories_created: usize,
}



/// Perform a copy from `source_directory_path` to `validated_target_path`.
///
/// This `*_unchecked` function does not validate the source and target paths (i.e. expects them to be already validated).
///
/// For more details, see [`copy_directory`].
pub(crate) fn copy_directory_unchecked(
    prepared_copy: PreparedDirectoryCopy,
    options: DirectoryCopyOptions,
) -> Result<FinishedDirectoryCopy, DirectoryError> {
    let should_overwrite_files = options
        .target_directory_rule
        .should_overwrite_existing_files();
    let should_overwrite_directories = options
        .target_directory_rule
        .should_overwrite_existing_directories();

    // So we have the entire queue of operations and we've made sure there are no collisions we should worry about.
    // What's left is performing the copy and directory create operations *precisely in the defined order*.
    // If we ignore the order, we could get into situations where
    // a directory doesn't exist yet, but we would want to copy a file into it.

    let mut total_bytes_copied = 0;
    let mut num_files_copied = 0;
    let mut num_directories_created = 0;

    // Create root target directory if needed.
    if !prepared_copy.validated_target.exists {
        fs::create_dir_all(&prepared_copy.validated_target.directory_path)
            .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

        num_directories_created += 1;
    }

    // Execute all queued operations (copying files and creating directories).
    for operation in prepared_copy.required_operations {
        match operation {
            QueuedOperation::CopyFile {
                source_file_path: source_path,
                source_size_bytes,
                target_file_path: target_path,
            } => {
                if target_path.exists() {
                    if !target_path.is_file() {
                        return Err(DirectoryError::TargetItemAlreadyExists {
                            path: target_path.clone(),
                        });
                    }

                    if !should_overwrite_files {
                        return Err(DirectoryError::TargetItemAlreadyExists {
                            path: target_path.clone(),
                        });
                    }
                }

                copy_file(
                    source_path,
                    &target_path,
                    FileCopyOptions {
                        overwrite_existing: should_overwrite_files,
                        skip_existing: false,
                    },
                )
                .map_err(|error| match error {
                    FileError::NotFound => DirectoryError::SourceContentsInvalid,
                    FileError::NotAFile => DirectoryError::SourceContentsInvalid,
                    FileError::UnableToAccessSourceFile { error } => {
                        DirectoryError::UnableToAccessSource { error }
                    }
                    FileError::AlreadyExists => DirectoryError::TargetItemAlreadyExists {
                        path: target_path.clone(),
                    },
                    FileError::UnableToAccessTargetFile { error } => {
                        DirectoryError::UnableToAccessTarget { error }
                    }
                    FileError::SourceAndTargetAreTheSameFile => {
                        DirectoryError::InvalidTargetDirectoryPath
                    }
                    FileError::OtherIoError { error } => DirectoryError::OtherIoError { error },
                })?;

                num_files_copied += 1;
                total_bytes_copied += source_size_bytes;
            }
            QueuedOperation::CreateDirectory {
                source_size_bytes,
                target_directory_path,
            } => {
                if target_directory_path.exists() {
                    if !target_directory_path.is_dir() {
                        return Err(DirectoryError::TargetItemAlreadyExists {
                            path: target_directory_path.clone(),
                        });
                    }

                    if !should_overwrite_directories {
                        return Err(DirectoryError::TargetItemAlreadyExists {
                            path: target_directory_path.clone(),
                        });
                    }

                    continue;
                }

                fs::create_dir(target_directory_path)
                    .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

                num_directories_created += 1;
                total_bytes_copied += source_size_bytes;
            }
        };
    }

    Ok(FinishedDirectoryCopy {
        total_bytes_copied,
        num_files_copied,
        num_directories_created,
    })
}


/// Copy a directory from `source_directory_path` to `target_directory_path`.
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `target_directory_path` will be created.
///
/// ### Target directory
/// Depending on the [`options.target_directory_rule`][DirectoryCopyOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - with [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see fields).
///
/// If the specified target directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidTargetDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::TargetDirectoryIsNotEmpty`]`)` is returned (depending on the rule).
///
/// ### Copy depth
/// Depending on the [`DirectoryCopyOptions::maximum_copy_depth`] option, calling this function means copying:
/// - `Some(0)` -- a single directory and its direct descendants (files and direct directories, but *not their contents*, i.e. just empty directories),
/// - `Some(>=1)` -- files and subdirectories (and their files and directories, etc.) up to a certain depth limit (e.g. `Some(1)` copies direct descendants as well as one layer deeper),
/// - `None` -- the entire subtree. **This is probably what you want most of the time**.
///
/// ## Symbolic links
/// - If the `source_directory_path` directory contains a symbolic link to a file,
/// the contents of the file it points to will be copied
/// into the corresponding subpath inside `target_file_path`
/// (same behaviour as `cp` without `-P` on Unix, i.e. link is followed, but not preserved).
/// - If the `source_directory_path` directory contains a symbolic link to a directory,
/// the directory and its contents will be copied as normal - the links will be followed, but not preserved.
///
/// ### Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`FinishedDirectoryCopy`].
pub fn copy_directory<S, T>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryCopyOptions,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let prepared_copy = PreparedDirectoryCopy::prepare(
        source_directory_path.as_ref(),
        target_directory_path.as_ref(),
        options.maximum_copy_depth,
        &options.target_directory_rule,
    )?;

    copy_directory_unchecked(prepared_copy, options)
}


/// Describes a directory copy operation.
///
/// Used in progress reporting in [`copy_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DirectoryCopyOperation {
    /// Describes a directory creation operation.
    CreatingDirectory { target_path: PathBuf },
    /// Describes a file being copied.
    /// For more precise copying progress, see the `progress` field.
    CopyingFile {
        target_path: PathBuf,
        progress: FileProgress,
    },
}


/// Represents the progress of copying a directory.
///
/// Used to report directory copying progress to a user-provided closure, see [`copy_directory_with_progress`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryCopyProgress {
    /// Amount of bytes that need to be copied for the directory copy to be complete.
    pub bytes_total: u64,

    /// Amount of bytes that have been copied so far.
    pub bytes_finished: u64,

    /// Number of files that have been copied so far.
    pub files_copied: usize,

    /// Number of directories that have been created so far.
    pub directories_created: usize,

    /// The current operation being performed.
    pub current_operation: DirectoryCopyOperation,

    /// The index of the current operation (starts at `0`, goes up to (including) `total_operations - 1`).
    pub current_operation_index: isize,

    /// The total amount of operations that need to be performed to copy the requested directory.
    ///
    /// A single operation is either copying a file or creating a directory, see [`DirectoryCopyOperation`].
    pub total_operations: isize,
}

impl DirectoryCopyProgress {
    /// Update the current [`DirectoryCopyOperation`] with the given closure.
    /// After updating the operation, this function calls the given progress handler.
    fn update_operation_and_emit<M, F>(&mut self, mut modifer_closure: M, progress_handler: &mut F)
    where
        M: FnMut(&mut Self),
        F: FnMut(&DirectoryCopyProgress),
    {
        modifer_closure(self);

        progress_handler(self);
    }

    /// Replace the current [`DirectoryCopyOperation`] with the next one (incrementing the operation index).
    /// After updating the operation, this function calls the given progress handler.
    fn set_next_operation_and_emit<F>(
        &mut self,
        operation: DirectoryCopyOperation,
        progress_handler: &mut F,
    ) where
        F: FnMut(&DirectoryCopyProgress),
    {
        self.current_operation_index += 1;
        self.current_operation = operation;

        progress_handler(self)
    }
}



/// Options that influence the [`copy_directory_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyWithProgressOptions {
    /// Specifies whether you allow the target directory to exist before copying and whether it must be empty or not.
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// target files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`TargetDirectoryRule`] for more details and examples.
    pub target_directory_rule: TargetDirectoryRule,

    /// Maximum depth of the source directory to copy.
    ///
    /// - `None` indicates no limit.
    /// - `Some(0)` means a directory copy operation that copies only the files and
    ///   creates directories directly in the root directory and doesn't scan any subdirectories.
    /// - `Some(1)` includes the root directory's contents and one level of its subdirectories.
    pub maximum_copy_depth: Option<usize>,

    /// Internal buffer size (for both reading and writing) when copying files,
    /// defaults to 64 KiB.
    pub buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
    ///
    /// *Note that the interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for DirectoryCopyWithProgressOptions {
    fn default() -> Self {
        Self {
            target_directory_rule: TargetDirectoryRule::default(),
            maximum_copy_depth: None,
            // 64 KiB
            buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}



/// Given [`QueuedOperation::CopyFile`] data, this function
/// copies the given file with progress information.
///
/// The function respects given `options` (e.g. returning an error
/// if the file already exists if configured to do so).
fn execute_copy_file_operation_with_progress<F>(
    source_path: PathBuf,
    source_size_bytes: u64,
    target_path: PathBuf,
    options: &DirectoryCopyWithProgressOptions,
    progress: &mut DirectoryCopyProgress,
    progress_handler: &mut F,
) -> Result<(), DirectoryError>
where
    F: FnMut(&DirectoryCopyProgress),
{
    let should_overwrite_files = options
        .target_directory_rule
        .should_overwrite_existing_files();

    if target_path.exists() {
        if !target_path.is_file() {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_path.clone(),
            });
        }

        if !should_overwrite_files {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_path.clone(),
            });
        }
    }


    progress.set_next_operation_and_emit(
        DirectoryCopyOperation::CopyingFile {
            target_path: target_path.clone(),
            progress: FileProgress {
                bytes_finished: 0,
                bytes_total: source_size_bytes,
            },
        },
        progress_handler,
    );

    let mut did_update_with_fresh_total = false;
    let bytes_copied_before = progress.bytes_finished;

    let num_bytes_copied = copy_file_with_progress(
        source_path,
        &target_path,
        FileCopyWithProgressOptions {
            overwrite_existing: should_overwrite_files,
            skip_existing: false,
            buffer_size: options.buffer_size,
            progress_update_byte_interval: options.progress_update_byte_interval,
        },
        |new_file_progress| progress.update_operation_and_emit(
                |progress| {
                    if let DirectoryCopyOperation::CopyingFile {
                        progress: file_progress,
                        ..
                    } = &mut progress.current_operation
                    {
                        // It is somewhat possible that a file is written to between the scanning phase and copying.
                        // In that case, it is *possible* that the file size changes, which means we should listen
                        // to the size `copy_file_with_progress` is reporting. There is no point
                        // to doing this each update, so we do it only once.
                        if !did_update_with_fresh_total {
                            file_progress.bytes_total = new_file_progress.bytes_total;
                            did_update_with_fresh_total = true;
                        }

                        file_progress.bytes_finished = new_file_progress.bytes_finished;
                        progress.bytes_finished =
                            bytes_copied_before + file_progress.bytes_finished;
                    } else {
                        panic!(
                            "bug: `progress.current_operation` miraculously doesn't match CopyingFile"
                        );
                    }
                },
                progress_handler,
            )
    )
    .map_err(|error| match error {
        FileError::NotFound => DirectoryError::SourceContentsInvalid,
        FileError::NotAFile => DirectoryError::SourceContentsInvalid,
        FileError::UnableToAccessSourceFile { error } => {
            DirectoryError::UnableToAccessSource { error }
        }
        FileError::AlreadyExists => DirectoryError::TargetItemAlreadyExists {
            path: target_path.clone(),
        },
        FileError::UnableToAccessTargetFile { error } => {
            DirectoryError::UnableToAccessTarget { error }
        }
        FileError::SourceAndTargetAreTheSameFile => DirectoryError::InvalidTargetDirectoryPath,
        FileError::OtherIoError { error } => DirectoryError::OtherIoError { error },
    })?;

    progress.files_copied += 1;

    debug_assert_eq!(
        progress.bytes_finished - num_bytes_copied,
        bytes_copied_before,
        "bug: reported incorrect amount of copied bytes"
    );

    Ok(())
}

/// Given [`QueuedOperation::CreateDirectory`] data, this function
/// creates the given directory with progress information.
///
/// If the directory already exists, no action
/// is taken, unless the given options indicate that to be an error
/// ([`overwrite_existing_subdirectories`][DirectoryCopyWithProgressOptions::overwrite_existing_subdirectories]).
///
/// If the given path exists, but is not a directory, an error is returned as well.
fn execute_create_directory_operation_with_progress<F>(
    target_directory_path: PathBuf,
    source_size_bytes: u64,
    should_overwrite_directories: bool,
    progress: &mut DirectoryCopyProgress,
    progress_handler: &mut F,
) -> Result<(), DirectoryError>
where
    F: FnMut(&DirectoryCopyProgress),
{
    if target_directory_path.exists() {
        if !target_directory_path.is_dir() {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_directory_path.clone(),
            });
        }

        if !should_overwrite_directories {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_directory_path.clone(),
            });
        }

        return Ok(());
    }

    progress.set_next_operation_and_emit(
        DirectoryCopyOperation::CreatingDirectory {
            target_path: target_directory_path.clone(),
        },
        progress_handler,
    );

    fs::create_dir(target_directory_path)
        .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

    progress.directories_created += 1;
    progress.bytes_finished += source_size_bytes;

    Ok(())
}



/// Perform a with-progress copy from `source_directory_path` to `validated_target`.
///
/// This `*_unchecked` function does not validate the source and target paths (i.e. expects them to be already validated).
///
/// For more details, see [`copy_directory_with_progress`].
pub(crate) fn perform_prepared_copy_directory_with_progress<F>(
    prepared_copy: PreparedDirectoryCopy,
    options: DirectoryCopyWithProgressOptions,
    mut progress_handler: F,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    F: FnMut(&DirectoryCopyProgress),
{
    let allows_existing_target_directory = options
        .target_directory_rule
        .allows_existing_target_directory();
    let should_overwrite_directories = options
        .target_directory_rule
        .should_overwrite_existing_directories();


    let target = &prepared_copy.validated_target;

    // Create root target directory if needed.
    let mut progress = if target.exists {
        if !allows_existing_target_directory && !should_overwrite_directories {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target.directory_path.clone(),
            });
        }

        DirectoryCopyProgress {
            bytes_total: prepared_copy.bytes_total,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            // This is an invisible operation - we don't emit this progress struct at all,
            // but we do need something here before the next operation starts.
            current_operation: DirectoryCopyOperation::CreatingDirectory {
                target_path: PathBuf::new(),
            },
            current_operation_index: -1,
            total_operations: prepared_copy.required_operations.len() as isize,
        }
    } else {
        // This time we actually emit this root directory creation progress.
        let mut progress = DirectoryCopyProgress {
            bytes_total: prepared_copy.bytes_total,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            current_operation: DirectoryCopyOperation::CreatingDirectory {
                target_path: target.directory_path.clone(),
            },
            current_operation_index: 0,
            total_operations: prepared_copy.required_operations.len() as isize + 1,
        };

        progress_handler(&progress);

        fs::create_dir_all(&target.directory_path)
            .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

        progress.directories_created += 1;

        progress
    };


    // Execute queued directory copy operations.
    for operation in prepared_copy.required_operations {
        match operation {
            QueuedOperation::CopyFile {
                source_file_path: source_path,
                source_size_bytes,
                target_file_path: target_path,
            } => execute_copy_file_operation_with_progress(
                source_path,
                source_size_bytes,
                target_path,
                &options,
                &mut progress,
                &mut progress_handler,
            )?,
            QueuedOperation::CreateDirectory {
                source_size_bytes,
                target_directory_path,
            } => execute_create_directory_operation_with_progress(
                target_directory_path,
                source_size_bytes,
                should_overwrite_directories,
                &mut progress,
                &mut progress_handler,
            )?,
        }
    }

    // One last progress update - everything should be done at this point.
    progress_handler(&progress);

    Ok(FinishedDirectoryCopy {
        total_bytes_copied: progress.bytes_finished,
        num_files_copied: progress.files_copied,
        num_directories_created: progress.directories_created,
    })
}


/// Copy an entire directory from `source_directory_path` to `target_directory_path`
/// (including progress reporting).
///
/// Things to consider:
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///   If needed, `target_directory_path` will be created.
///
///
/// ### Target directory
/// Depending on the [`options.target_directory_rules`][DirectoryCopyOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - with [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - with [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - with [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see variant's fields).
///
/// If the specified target directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidTargetDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::TargetDirectoryIsNotEmpty`]`)` is returned (depending on the rule).
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
/// Depending on the [`options.maximum_copy_depth`] option, calling this function means copying:
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
    target_directory_path: T,
    options: DirectoryCopyWithProgressOptions,
    progress_handler: F,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryCopyProgress),
{
    let prepared_copy = PreparedDirectoryCopy::prepare(
        source_directory_path.as_ref(),
        target_directory_path.as_ref(),
        options.maximum_copy_depth,
        &options.target_directory_rule,
    )?;


    perform_prepared_copy_directory_with_progress(prepared_copy, options, progress_handler)
}
