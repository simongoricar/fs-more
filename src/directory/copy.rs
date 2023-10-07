use std::path::{Path, PathBuf};

use super::scan::is_directory_empty_unchecked;
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

/// Ensures the given source directory path is valid.
/// This means that:
/// - it exists,
/// - is a directory.
///
/// The returned path is a canonicalized version of the provided path.
pub(super) fn validate_source_directory_path(
    source_directory_path: &Path,
) -> Result<PathBuf, DirectoryError> {
    // Ensure the source directory path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `DirectoryError::NotFound` error.
    match source_directory_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(DirectoryError::SourceDirectoryNotFound);
            }
        }
        Err(error) => {
            return Err(DirectoryError::UnableToAccessSource { error });
        }
    }

    if !source_directory_path.is_dir() {
        return Err(DirectoryError::SourceDirectoryIsNotADirectory);
    }

    let canonicalized_path = std::fs::canonicalize(source_directory_path)
        .map_err(|error| DirectoryError::OtherIoError { error })?;

    Ok(dunce::simplified(&canonicalized_path).to_path_buf())
}

/// Information about a validated target path (used in copying and moving directories).
pub(crate) struct ValidatedTargetPath {
    pub(crate) target_directory_path: PathBuf,
    pub(crate) target_directory_exists: bool,
    pub(crate) target_directory_is_empty: Option<bool>,
}

/// Ensures the given target directory path is valid:
/// This means either that:
/// - it exists (if `allow_existing_target_directory == true`, otherwise, this is an `Err`),
/// - it doesn't exist.
///
/// The returned tuple consists of:
/// - a `PathBuf`, which is a cleaned `target_directory_path` (with the `path_clean` library), and,
/// - a `bool`, which indicates whether the directory needs to be created.
pub(super) fn validate_target_directory_path(
    target_directory_path: &Path,
    target_directory_rules: &TargetDirectoryRule,
) -> Result<ValidatedTargetPath, DirectoryError> {
    let target_directory_exists = target_directory_path
        .try_exists()
        .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

    // If `target_directory_path` does not point to a directory,
    // but instead e.g. a file, we should abort.
    if target_directory_exists && !target_directory_path.is_dir() {
        return Err(DirectoryError::InvalidTargetDirectoryPath);
    }

    let is_empty = if target_directory_exists {
        is_directory_empty_unchecked(target_directory_path)
            .map(Some)
            .map_err(|error| DirectoryError::OtherIoError { error })?
    } else {
        None
    };


    match target_directory_rules {
        TargetDirectoryRule::DisallowExisting if target_directory_exists => {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_directory_path.to_path_buf(),
            });
        }
        TargetDirectoryRule::AllowEmpty if target_directory_exists => {
            if !is_empty.unwrap_or(true) {
                return Err(DirectoryError::TargetDirectoryIsNotEmpty);
            }
        }
        _ => {}
    };

    let clean_path = path_clean::clean(target_directory_path);

    Ok(ValidatedTargetPath {
        target_directory_path: clean_path,
        target_directory_exists,
        target_directory_is_empty: is_empty,
    })
}

pub(super) fn validate_source_target_directory_pair(
    source_directory_path: &Path,
    target_directory_path: &Path,
) -> Result<(), DirectoryError> {
    // Ensure `target_directory_path` isn't equal or a subdirectory of `source_directory_path`˙.
    if target_directory_path.starts_with(source_directory_path) {
        return Err(DirectoryError::InvalidTargetDirectoryPath);
    }

    Ok(())
}

/// Specifies whether you allow the target directory to exist
/// before copying or moving files or directories into it.
///
/// If you allow the target directory to exist, you can also specify whether it must be empty;
/// if not, you may also specify whether you allow files and directories to be overwritten.
///
/// ## Defaults
/// [`Default`] is implemented for this enum. The default value is [`TargetDirectoryRule::AllowEmpty`].
///
/// ## Examples
/// If you want the associated directory copying or moving function to
/// *return an error if the target directory already exists*, use [`TargetDirectoryRule::DisallowExisting`];
///
/// If you want to copy into an *existing empty target directory*, you should use [`TargetDirectoryRule::AllowEmpty`]
/// (this rule *does not require* the target directory to exist and will create one if missing).
///
/// If the target directory could already exist and have some files or directories in it, you can use the following rule:
/// ```rust
/// # use fs_more::directory::TargetDirectoryRule;
/// let rules = TargetDirectoryRule::AllowNonEmpty {
///     overwrite_existing_subdirectories: false,
///     overwrite_existing_files: false,
/// };
/// ```
///
/// This will still not overwrite any overlapping files (i.e. a merge without overwrites will be performed).
///
/// If you want files and/or directories to be overwritten, you may set the flags for overwriting to `true`:
/// ```rust
/// # use fs_more::directory::TargetDirectoryRule;
/// let rules = TargetDirectoryRule::AllowNonEmpty {
///     overwrite_existing_subdirectories: true,
///     overwrite_existing_files: true,
/// };
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetDirectoryRule {
    /// Indicates the associated function should return an error if the target directory already exists.
    DisallowExisting,

    /// Indicates the associated function should return an error if the target directory
    /// exists *and is not empty*.
    AllowEmpty,

    /// Indicates that an existing non-empty target directory should not cause an error.
    AllowNonEmpty {
        /// If enabled, the associated function will return
        /// `Err(`[`DirectoryError::TargetItemAlreadyExists`][crate::error::DirectoryError::TargetItemAlreadyExists]`)`
        /// if a target directory or any of subdirectories that would otherwise need to be freshly created already exist.
        overwrite_existing_subdirectories: bool,

        /// If enabled, the associated function will return
        /// `Err(`[`DirectoryError::TargetItemAlreadyExists`][crate::error::DirectoryError::TargetItemAlreadyExists]`)`
        /// if a target file we would otherwise freshly create and copy into already exists.
        overwrite_existing_files: bool,
    },
}

impl Default for TargetDirectoryRule {
    fn default() -> Self {
        Self::AllowEmpty
    }
}

impl TargetDirectoryRule {
    /// Indicates whether this rule allows the target directory
    /// to exist before performing an operation.
    pub fn allows_existing_target_directory(&self) -> bool {
        !matches!(self, Self::DisallowExisting)
    }

    /// Indicates whether this rule allows existing files
    /// in the target directory to be overwritten with contents of the source.
    pub fn should_overwrite_existing_files(&self) -> bool {
        match self {
            TargetDirectoryRule::DisallowExisting => false,
            TargetDirectoryRule::AllowEmpty => false,
            TargetDirectoryRule::AllowNonEmpty {
                overwrite_existing_files,
                ..
            } => *overwrite_existing_files,
        }
    }

    /// Indicates whether this rule allows existing (sub)directories
    /// in the target directory to be "overwritten" with contents of the source (sub)directory.
    pub fn should_overwrite_existing_directories(&self) -> bool {
        match self {
            TargetDirectoryRule::DisallowExisting => false,
            TargetDirectoryRule::AllowEmpty => false,
            TargetDirectoryRule::AllowNonEmpty {
                overwrite_existing_subdirectories,
                ..
            } => *overwrite_existing_subdirectories,
        }
    }
}


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


/// Given a source root path, a target root path and the source path to rejoin,
/// this function takes the `source_path_to_rejoin`, removes the prefix provided by `source_root_path`
/// and repplies that relative path back onto the `target_root_path`.
///
/// Returns a [`DirectoryError::SubdirectoryEscapesRoot`] if the `source_path_to_rejoin`
/// is not a subpath of `source_root_path`. This function will not return any other error from
/// the [`DirectoryError`] struct.
///
/// ## Example
/// ```ignore
/// # use std::path::Path;
/// # use fs_more::directory::copy::rejoin_source_subpath_onto_target;
///
/// let root_a = Path::new("/hello/there");
/// let foo = Path::new("/hello/there/some/content");
/// let root_b = Path::new("/different/root");
///
/// assert_eq!(
///     rejoin_source_subpath_onto_target(
///         root_a,
///         foo,
///         root_b
///     ).unwrap(),
///     Path::new("/different/root/some/content")
/// );
/// ```
pub(crate) fn rejoin_source_subpath_onto_target(
    source_root_path: &Path,
    source_path_to_rejoin: &Path,
    target_root_path: &Path,
) -> Result<PathBuf, DirectoryError> {
    // Strip the source subdirectory path from the full source path
    // and place it on top of the target directory path.
    let source_relative_subdirectory_path = if source_root_path.eq(source_path_to_rejoin) {
        Path::new("")
    } else {
        source_path_to_rejoin
            .strip_prefix(source_root_path)
            .map_err(|_| DirectoryError::SubdirectoryEscapesRoot)?
    };

    Ok(target_root_path.join(source_relative_subdirectory_path))
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


/// Represents a file copy or directory creation operation.
///
/// For more details, see the [`build_directory_copy_queue`] function.
#[derive(Clone, Debug)]
enum QueuedOperation {
    CopyFile {
        source_file_path: PathBuf,
        source_size_bytes: u64,
        target_file_path: PathBuf,
    },
    CreateDirectory {
        source_size_bytes: u64,
        target_directory_path: PathBuf,
    },
}


/// Given a source and target directory as well as, optionally, a maximum copy depth,
/// this function builds a list of [`QueuedOperation`]s that are needed to fully,
/// or up to the `maximum_depth` limit, copy the source directory to the target directory.
///
/// The order of directory creation and file copying operations is such that
/// for any file in the list, its directory has previously been created
/// (its creation appears before it in the queue).
///
/// Note, however, that **the queued operations do not include creation of the `target_directory_root_path`
/// directory itself**, even if that is necessary in your case.
fn build_directory_copy_queue<S, T>(
    source_directory_root_path: S,
    target_directory_root_path: T,
    maximum_depth: Option<usize>,
) -> Result<Vec<QueuedOperation>, DirectoryError>
where
    S: Into<PathBuf>,
    T: Into<PathBuf>,
{
    let source_directory_root_path = source_directory_root_path.into();
    let target_directory_root_path = target_directory_root_path.into();

    let mut operation_queue: Vec<QueuedOperation> = Vec::new();


    // Scan the source directory and queue all copy and
    // directory create operations that need to happen.
    struct PendingDirectoryScan {
        source_directory_path: PathBuf,
        depth: usize,
    }

    let mut directory_scan_queue = Vec::new();
    directory_scan_queue.push(PendingDirectoryScan {
        source_directory_path: source_directory_root_path.clone(),
        depth: 0,
    });

    // Perform directory scans using a queue.
    while let Some(next_directory) = directory_scan_queue.pop() {
        // Scan the directory for its files and directories.
        // Files are queued for copying, directories are queued for creation.
        let directory_iterator = std::fs::read_dir(&next_directory.source_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

        for directory_item in directory_iterator {
            let directory_item =
                directory_item.map_err(|error| DirectoryError::UnableToAccessSource { error })?;

            let directory_item_source_path = directory_item.path();
            let directory_item_target_path = rejoin_source_subpath_onto_target(
                &source_directory_root_path,
                &directory_item_source_path,
                &target_directory_root_path,
            )?;

            let item_type = directory_item
                .file_type()
                .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

            if item_type.is_file() {
                let file_metadata = directory_item
                    .metadata()
                    .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                let file_size_in_bytes = file_metadata.len();

                operation_queue.push(QueuedOperation::CopyFile {
                    source_file_path: directory_item_source_path,
                    source_size_bytes: file_size_in_bytes,
                    target_file_path: directory_item_target_path,
                });
            } else if item_type.is_dir() {
                let directory_metadata = directory_item
                    .metadata()
                    .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                // Note that this is the size of the directory itself, not of its contents.
                let directory_size_in_bytes = directory_metadata.len();

                operation_queue.push(QueuedOperation::CreateDirectory {
                    source_size_bytes: directory_size_in_bytes,
                    target_directory_path: directory_item_target_path,
                });

                // If we haven't reached the maximum depth yet, we queue the directory for scanning.
                if let Some(maximum_depth) = maximum_depth {
                    if next_directory.depth < maximum_depth {
                        directory_scan_queue.push(PendingDirectoryScan {
                            source_directory_path: directory_item_source_path,
                            depth: next_directory.depth + 1,
                        });
                    }
                } else {
                    directory_scan_queue.push(PendingDirectoryScan {
                        source_directory_path: directory_item_source_path,
                        depth: next_directory.depth + 1,
                    });
                }
            } else if item_type.is_symlink() {
                // If the path is a symbolic link, we need to follow it and queue a copy from the destination file.
                // Can point to either a directory or a file.

                // Now we should retrieve the metadata of the target of the symbolic link
                // (unlike DirEntry::metadata, this metadata call *does* follow symolic links).
                let underlying_path = std::fs::canonicalize(&directory_item_source_path)
                    .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                let underlying_item_metadata = underlying_path
                    .metadata()
                    .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                if underlying_item_metadata.is_file() {
                    let underlying_file_size_in_bytes = underlying_item_metadata.len();

                    operation_queue.push(QueuedOperation::CopyFile {
                        source_file_path: underlying_path,
                        source_size_bytes: underlying_file_size_in_bytes,
                        target_file_path: directory_item_target_path,
                    });
                } else if underlying_item_metadata.is_dir() {
                    // Note that this is the size of the directory itself, not of its contents.
                    let underlying_directory_size_in_bytes = underlying_item_metadata.len();

                    operation_queue.push(QueuedOperation::CreateDirectory {
                        source_size_bytes: underlying_directory_size_in_bytes,
                        target_directory_path: directory_item_target_path,
                    });

                    // If we haven't reached the maximum depth yet, we queue the directory for scanning.
                    if let Some(maximum_depth) = maximum_depth {
                        if next_directory.depth < maximum_depth {
                            directory_scan_queue.push(PendingDirectoryScan {
                                source_directory_path: directory_item_source_path,
                                depth: next_directory.depth + 1,
                            });
                        }
                    } else {
                        directory_scan_queue.push(PendingDirectoryScan {
                            source_directory_path: directory_item_source_path,
                            depth: next_directory.depth + 1,
                        });
                    }
                }
            }
        }
    }

    Ok(operation_queue)
}

/// Given a list of queued operations, this function validates that
/// the files we'd be copying into or target directories we'd create don't exist yet
/// (or however the [`TargetDirectoryRule`] is configured).
fn check_operation_queue_for_collisions(
    queue: &[QueuedOperation],
    target_directory_rules: &TargetDirectoryRule,
) -> Result<(), DirectoryError> {
    let can_overwrite_files = target_directory_rules.should_overwrite_existing_files();
    let can_overwrite_directories = target_directory_rules.should_overwrite_existing_directories();

    if can_overwrite_files && can_overwrite_directories {
        // There is nothing to check, we can't have any collisions
        // if we allow everything to be overwritten.
        return Ok(());
    }

    for queue_item in queue {
        match queue_item {
            QueuedOperation::CopyFile {
                target_file_path, ..
            } if !can_overwrite_files => {
                if target_file_path.exists() {
                    return Err(DirectoryError::TargetItemAlreadyExists {
                        path: target_file_path.clone(),
                    });
                }
            }
            QueuedOperation::CreateDirectory {
                target_directory_path,
                ..
            } if !can_overwrite_directories => {
                if target_directory_path.exists() {
                    return Err(DirectoryError::TargetItemAlreadyExists {
                        path: target_directory_path.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(())
}


/// Perform a copy from `source_directory_path` to `validated_target_path`.
///
/// For more details, see [`copy_directory`].
pub(crate) fn copy_directory_unchecked<S>(
    source_directory_path: S,
    validated_target_path: ValidatedTargetPath,
    options: DirectoryCopyOptions,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    S: Into<PathBuf>,
{
    let source_directory_path: PathBuf = source_directory_path.into();
    let ValidatedTargetPath {
        target_directory_path,
        target_directory_exists,
        ..
    } = validated_target_path;

    let should_overwrite_files = options
        .target_directory_rule
        .should_overwrite_existing_files();
    let should_overwrite_directories = options
        .target_directory_rule
        .should_overwrite_existing_directories();

    // Initialize a queue of file copy or directory create operations.
    let operation_queue = build_directory_copy_queue(
        &source_directory_path,
        &target_directory_path,
        options.maximum_copy_depth,
    )?;

    // We should do a reasonable target directory file/directory collision check and return a TargetItemAlreadyExists early,
    // before we copy any file at all. This way the target directory stays intact as often as possible,
    // instead of returning an error after having copied some files already (which would be hard to reverse).
    // It's still possible that due to a race condition we don't catch a collision here yet,
    // but that should be very rare and is essentially unsolvable (unless there was
    // a robust rollback mechanism, which is out of scope for this project).

    // TODO Add a test for this (test when the error is returned).
    check_operation_queue_for_collisions(&operation_queue, &options.target_directory_rule)?;

    // So we've built the entire queue of operations and made sure there are no collisions we should worry about.
    // What's left is performing the copy and directory create operations *precisely in the defined order*.
    // If we ignore the order, we could get into situations where
    // a directory doesn't exist yet, but we would want to copy a file into it.
    // Instead, the `build_directory_copy_queue` takes care of the correct operation order.

    let mut total_bytes_copied = 0;
    let mut num_files_copied = 0;
    let mut num_directories_created = 0;

    // TODO feature flag to use fs-err?

    // Create root target directory if needed.
    if !target_directory_exists {
        std::fs::create_dir_all(target_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

        num_directories_created += 1;
    }

    // Execute all queued operations (copying files and creating directories).
    for operation in operation_queue {
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

                std::fs::create_dir(target_directory_path)
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
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///
/// ### Target directory
/// Depending on the [`options.target_directory_rule`][DirectoryCopyOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see fields).
///
/// If the specified target directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidTargetDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::TargetDirectoryIsNotEmpty`]`)` is returned (depending on the rule).
///
/// ### Copy depth
/// Depending on the [`DirectoryCopyOptions::maximum_copy_depth`] option, calling this function means copying:
/// - `Some(0)` -- a single directory and its direct descendants (files and direct directories, but *not their contents*, i.e. just empty directories),
/// - `Some(1+)` -- files and subdirectories (and their files and directories, etc.) up to a certain depth limit (e.g. `Some(1)` copies direct descendants as well as one layer deeper),
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
    let source_directory_path = validate_source_directory_path(source_directory_path.as_ref())?;
    let validated_target_path = validate_target_directory_path(
        target_directory_path.as_ref(),
        &options.target_directory_rule,
    )?;

    validate_source_target_directory_pair(
        &source_directory_path,
        &validated_target_path.target_directory_path,
    )?;

    copy_directory_unchecked(
        source_directory_path,
        validated_target_path,
        options,
    )
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

    /// The index of the current operation (starts at `0`, goes to `total_operations - 1`).
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

    /// Internal buffer size (for both reading and writing) when copying filea,
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

    std::fs::create_dir(target_directory_path)
        .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

    progress.directories_created += 1;
    progress.bytes_finished += source_size_bytes;

    Ok(())
}


/// Copy an entire directory from `source_directory_path` to `target_directory_path`.
///
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///
/// ### Target directory
/// Depending on the [`options.target_directory_rules`][DirectoryCopyOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see variant's fields).
///
/// If the specified target directory rule does not hold,
/// `Err(`[`DirectoryError::InvalidTargetDirectoryPath`]`)` or
/// `Err(`[`DirectoryError::TargetDirectoryIsNotEmpty`]`)` is returned (depending on the rule).
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
/// For more information about update frequency of specifically file copy updates, refer to the `Progress reporting` section
/// of the [`copy_file_with_progress`][crate::file::copy_file_with_progress] function.
///
/// ## Copy depth
/// Depending on the [`options.maximum_copy_depth`] option, calling this function means copying:
/// - `Some(0)` -- a single directory and its direct descendants (files and direct directories, but *not their contents*, i.e. just empty directories),
/// - `Some(1+)` -- files and subdirectories (and their files and directories, etc.) up to a certain depth limit (e.g. `Some(1)` copies direct descendants as well as one layer deeper),
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
/// ## Return value
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied, see [`FinishedDirectoryCopy`].
pub fn copy_directory_with_progress<S, T, F>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryCopyWithProgressOptions,
    mut progress_handler: F,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&DirectoryCopyProgress),
{
    // TODO Test how this and copy_directory handle symbolic links to directories.
    let allows_existing_target_directory = options
        .target_directory_rule
        .allows_existing_target_directory();
    let should_overwrite_directories = options
        .target_directory_rule
        .should_overwrite_existing_directories();

    let source_directory_path = validate_source_directory_path(source_directory_path.as_ref())?;
    let ValidatedTargetPath {
        target_directory_path,
        target_directory_exists,
        ..
    } = validate_target_directory_path(
        target_directory_path.as_ref(),
        &options.target_directory_rule,
    )?;

    validate_source_target_directory_pair(&source_directory_path, &target_directory_path)?;

    // Initialize a queue of file copy or directory create operations.
    let operation_queue = build_directory_copy_queue(
        &source_directory_path,
        &target_directory_path,
        options.maximum_copy_depth,
    )?;

    // TODO Add a test for this (test when the error is returned).
    check_operation_queue_for_collisions(&operation_queue, &options.target_directory_rule)?;

    let bytes_total = operation_queue
        .iter()
        .map(|item| match item {
            QueuedOperation::CopyFile {
                source_size_bytes, ..
            } => *source_size_bytes,
            QueuedOperation::CreateDirectory {
                source_size_bytes, ..
            } => *source_size_bytes,
        })
        .sum::<u64>();

    // Create root target directory if needed.
    let mut progress = if target_directory_exists {
        if !allows_existing_target_directory && !should_overwrite_directories {
            return Err(DirectoryError::TargetItemAlreadyExists {
                path: target_directory_path.to_path_buf(),
            });
        }

        DirectoryCopyProgress {
            bytes_total,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            // This is a bogus operation - we don't emit this progress,
            // but we need something here before the next operation starts.
            current_operation: DirectoryCopyOperation::CreatingDirectory {
                target_path: PathBuf::new(),
            },
            current_operation_index: -1,
            total_operations: operation_queue.len() as isize,
        }
    } else {
        // This time we actually emit this root directory creation progress.
        let mut progress = DirectoryCopyProgress {
            bytes_total,
            bytes_finished: 0,
            files_copied: 0,
            directories_created: 0,
            current_operation: DirectoryCopyOperation::CreatingDirectory {
                target_path: target_directory_path.to_path_buf(),
            },
            current_operation_index: 0,
            total_operations: operation_queue.len() as isize + 1,
        };

        progress_handler(&progress);

        std::fs::create_dir_all(target_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;

        progress.directories_created += 1;

        progress
    };


    for operation in operation_queue {
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


#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn properly_rejoin_source_subpath_onto_target() {
        let root_a = Path::new("/hello/there");
        let foo = Path::new("/hello/there/some/content");
        let root_b = Path::new("/different/root");

        assert_eq!(
            rejoin_source_subpath_onto_target(root_a, foo, root_b).unwrap(),
            Path::new("/different/root/some/content"),
            "rejoin_source_subpath_onto_target did not rejoin the path properly."
        );
    }

    #[test]
    fn error_on_subpath_not_being_under_source_root() {
        let root_a = Path::new("/hello/there");
        let foo = Path::new("/completely/different/path");
        let root_b = Path::new("/different/root");

        let rejoin_result = rejoin_source_subpath_onto_target(root_a, foo, root_b);

        assert!(
            rejoin_result.is_err(),
            "rejoin_source_subpath_onto_target did not return Err when \
            the source path to rejoin wasn't under the source root path"
        );

        let rejoin_err = rejoin_result.unwrap_err();

        assert_matches!(
            rejoin_err,
            DirectoryError::SubdirectoryEscapesRoot,
            "rejoin_source_subpath_onto_target did not return Err with SubdirectoryEscapesRoot, but {}",
            rejoin_err
        );
    }
}
