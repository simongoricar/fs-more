#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "fs-err")]
use fs_err as fs;

use super::{common::TargetDirectoryRule, is_directory_empty_unchecked};
use crate::{directory::common::rejoin_source_subpath_onto_target, error::DirectoryError};

/// Represents a file copy or directory creation operation.
///
/// For more details, see the [`build_directory_copy_queue`] function.
#[derive(Clone, Debug)]
pub(crate) enum QueuedOperation {
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

    let canonicalized_path = fs::canonicalize(source_directory_path)
        .map_err(|error| DirectoryError::OtherIoError { error })?;

    Ok(dunce::simplified(&canonicalized_path).to_path_buf())
}

/// Information about a validated target path (used in copying and moving directories).
///
/// "Valid" in this context means that it respects the user-provided `options`,
/// see [`validate_target_directory_path`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ValidatedTargetPath {
    pub(crate) directory_path: PathBuf,
    pub(crate) exists: bool,
    pub(crate) is_empty_directory: Option<bool>,
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
        directory_path: clean_path,
        exists: target_directory_exists,
        is_empty_directory: is_empty,
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
        let directory_iterator = fs::read_dir(&next_directory.source_directory_path)
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
                let underlying_path = fs::canonicalize(&directory_item_source_path)
                    .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                let underlying_item_metadata = fs::metadata(&underlying_path)
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
    let can_overwrite_files = target_directory_rules.allows_overwriting_existing_files();
    let can_overwrite_directories =
        target_directory_rules.allows_overwriting_existing_directories();

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



/// An auxiliary struct that prepares for a directory copy (with progress).
///
/// Start by calling [`Self::prepare`].
pub(crate) struct PreparedDirectoryCopy {
    /// What the copy target is. This is a validated path.
    ///
    /// "Valid" in this context means that it respects the user-provided `options`,
    /// see [`validate_target_directory_path`].
    pub(crate) validated_target: ValidatedTargetPath,

    /// An array of required file copy and directory create operations
    /// that togeher form a full directory copy.
    pub(crate) required_operations: Vec<QueuedOperation>,

    /// How many bytes will need to be copied (i.e. the source directory size).
    pub(crate) bytes_total: u64,
}


impl PreparedDirectoryCopy {
    /// Prepare for a new directory copy.
    /// This includes validating the source and target paths and preparing the operation queue.
    pub fn prepare(
        source_directory_path: &Path,
        target_directory_path: &Path,
        maximum_copy_depth: Option<usize>,
        target_directory_rule: &TargetDirectoryRule,
    ) -> Result<Self, DirectoryError> {
        let (source_directory_path, validated_target) = Self::validate_source_and_target_path_pair(
            source_directory_path,
            target_directory_path,
            target_directory_rule,
        )?;

        Self::prepare_with_validated(
            source_directory_path,
            validated_target,
            maximum_copy_depth,
            target_directory_rule,
        )
    }

    /// Prepare for a new directory copy.
    /// This initialization variant expects the target path to already be validated.
    ///
    /// This preparation therefore only includes preparing the operation queue.
    pub fn prepare_with_validated(
        validated_source_directory_path: PathBuf,
        validated_target: ValidatedTargetPath,
        maximum_copy_depth: Option<usize>,
        target_directory_rule: &TargetDirectoryRule,
    ) -> Result<Self, DirectoryError> {
        let operations = Self::prepare_directory_operations(
            validated_source_directory_path.clone(),
            validated_target.directory_path.clone(),
            maximum_copy_depth,
            target_directory_rule,
        )?;

        let bytes_total = Self::calculate_total_bytes_to_be_copied(&operations);


        Ok(Self {
            validated_target,
            required_operations: operations,
            bytes_total,
        })
    }

    fn calculate_total_bytes_to_be_copied(queued_operations: &[QueuedOperation]) -> u64 {
        queued_operations
            .iter()
            .map(|item| match item {
                QueuedOperation::CopyFile {
                    source_size_bytes, ..
                } => *source_size_bytes,
                QueuedOperation::CreateDirectory {
                    source_size_bytes, ..
                } => *source_size_bytes,
            })
            .sum::<u64>()
    }

    fn validate_source_and_target_path_pair(
        source_directory_path: &Path,
        target_directory_path: &Path,
        target_directory_rule: &TargetDirectoryRule,
    ) -> Result<(PathBuf, ValidatedTargetPath), DirectoryError> {
        let source_directory_path = validate_source_directory_path(source_directory_path)?;
        let validated_target =
            validate_target_directory_path(target_directory_path, target_directory_rule)?;

        validate_source_target_directory_pair(
            &source_directory_path,
            &validated_target.directory_path,
        )?;

        Ok((source_directory_path, validated_target))
    }

    fn prepare_directory_operations(
        source_directory_path: PathBuf,
        target_directory_path: PathBuf,
        maximum_copy_depth: Option<usize>,
        target_directory_rule: &TargetDirectoryRule,
    ) -> Result<Vec<QueuedOperation>, DirectoryError> {
        // Initialize a queue of file copy or directory create operations.
        let copy_queue = build_directory_copy_queue(
            source_directory_path,
            target_directory_path,
            maximum_copy_depth,
        )?;

        // We should do a reasonable target directory file/directory collision check and return a TargetItemAlreadyExists early,
        // before we copy any file at all. This way the target directory stays intact as often as possible,
        // instead of returning an error after having copied some files already (which would be hard to reverse).
        // It's still possible that due to a race condition we don't catch a collision here yet,
        // but that should be very rare and is essentially unsolvable (unless there was
        // a robust rollback mechanism, which is out of scope for this project).

        check_operation_queue_for_collisions(&copy_queue, target_directory_rule)?;

        Ok(copy_queue)
    }
}
