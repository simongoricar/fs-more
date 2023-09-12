use std::path::{Path, PathBuf};

use crate::{
    error::{DirectoryError, FileError},
    file::{copy_file, FileCopyOptions},
};

/// Options that influence the [`copy_directory`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DirectoryCopyOptions {
    /// Specifies whether you allow the target directory to exist before copying.
    /// If `false`, it will be created as part of the directory copy operation.
    pub allow_existing_target_directory: bool,

    /// Specifies whether to allow target subdirectories to already exist
    /// when copying.
    ///
    /// Only has an effect / makes sense if `allow_existing_target_directory` is `true`.
    pub overwrite_existing_subdirectories: bool,

    /// Specifies whether to overwrite an existing target file if it already exists before copying.
    ///
    /// Only has an effect / makes sense if `allow_existing_target_directory` is `true`.
    pub overwrite_existing_files: bool,

    /// Maximum depth of the source directory to copy.
    ///
    /// - `None` indicates no limit.
    /// - `Some(0)` means a directory copy operation that copies only the files and
    ///   creates directories directly in the root directory and doesn't scan any subdirectories.
    /// - `Some(1)` includes the root directory's contents and one level of its subdirectories.
    pub maximum_copy_depth: Option<usize>,
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
fn rejoin_source_subpath_onto_target(
    source_root_path: &Path,
    source_path_to_rejoin: &Path,
    target_root_path: &Path,
) -> Result<PathBuf, DirectoryError> {
    // Strip the source subdirectory path from the full source path
    // and place it on top of the target directory path.
    let source_relative_subdirectory_path =
        if source_root_path.eq(source_path_to_rejoin) {
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

    /// Amount of files copied when copying the directory.
    pub num_files_copied: usize,

    /// Amount of directories created when copying the directory.
    pub num_directories_created: usize,
}


/// Represents a file copy or directory creation operation.
///
/// For more details, see the [`build_directory_copy_queue`] function.
enum QueuedOperation {
    CopyFile {
        source_path: PathBuf,
        source_size_bytes: u64,
        target_path: PathBuf,
    },
    CreateDirectory {
        target_directory_path: PathBuf,
    },
}


/// Given a source and target directory as well as, optionally, a maximum copy depth,
/// this function builds a list of [`QueuedOperation`]s that are needed to fully
/// (or up to the `maximum_depth` limit)
/// copy the source directory to the target directory.
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
    let mut directory_scan_queue = Vec::new();

    struct PendingDirectoryScan {
        source_directory_path: PathBuf,
        depth: usize,
    }

    // Push the initial (root) directory.
    operation_queue.push(QueuedOperation::CreateDirectory {
        target_directory_path: target_directory_root_path.clone(),
    });
    directory_scan_queue.push(PendingDirectoryScan {
        source_directory_path: source_directory_root_path.clone(),
        depth: 0,
    });

    // Perform directory scans using a queue.
    while !directory_scan_queue.is_empty() {
        let next_directory = directory_scan_queue.pop().expect(
            "BUG: Can't pop item from Vec even though is_empty was false.",
        );

        // Scan the directory for its files and directories.
        // Files are queued for copying, directories are queued for creation.
        let directory_iterator = std::fs::read_dir(
            &next_directory.source_directory_path,
        )
        .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

        for directory_item in directory_iterator {
            let directory_item = directory_item.map_err(|error| {
                DirectoryError::UnableToAccessSource { error }
            })?;

            let directory_item_source_path = directory_item.path();
            let directory_item_target_path = rejoin_source_subpath_onto_target(
                &source_directory_root_path,
                &directory_item_source_path,
                &target_directory_root_path,
            )?;

            let item_type = directory_item.file_type().map_err(|error| {
                DirectoryError::UnableToAccessSource { error }
            })?;

            if item_type.is_file() {
                let file_metadata =
                    directory_item.metadata().map_err(|error| {
                        DirectoryError::UnableToAccessSource { error }
                    })?;

                let file_size_in_bytes = file_metadata.len();

                operation_queue.push(QueuedOperation::CopyFile {
                    source_path: directory_item_source_path,
                    source_size_bytes: file_size_in_bytes,
                    target_path: directory_item_target_path,
                });
            } else if item_type.is_dir() {
                operation_queue.push(QueuedOperation::CreateDirectory {
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

    todo!();
}


/// Copy a directory. This can, depending on the [`maximum_copy_depth`][DirectoryCopyOptions::maximum_copy_depth] option, mean copying:
/// - either a single directory and its files (and direct directories, which will end up empty)  -- set the option to `Some(0)`),
/// - files and subdirectories (and their contents) up to a certain depth limit  -- set the option to `Some(1)` or more), or
/// - the entire subtree (which is probably what you want most of the time) -- set the option to `None`.
///
/// `source_directory_path` must point to an existing directory path.
///
/// `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
/// Barring explicit options, the path must point to a non-existing path.
/// For more information, see these three options:
/// - [`allow_existing_target_directory`][DirectoryCopyOptions::allow_existing_target_directory],
/// - [`overwrite_existing_subdirectories`][DirectoryCopyOptions::overwrite_existing_subdirectories], and
/// - [`overwrite_existing_files`][DirectoryCopyOptions::overwrite_existing_files].
///
/// Upon success, the function returns information about the files and directories that were copied or created
/// as well as the total amount of bytes copied.
///
/// *Warning:* this function does not follow symbolic links.
pub fn copy_directory<S, T>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryCopyOptions,
) -> Result<FinishedDirectoryCopy, DirectoryError>
where
    S: Into<PathBuf>,
    T: AsRef<Path>,
{
    let source_directory_path =
        std::fs::canonicalize(source_directory_path.into())
            .map_err(|error| DirectoryError::PathError { error })?;

    let target_directory_path = target_directory_path.as_ref();

    // Ensure the source directory path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `DirectoryError::NotFound` error.
    match source_directory_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(DirectoryError::SourceRootDirectoryNotFound);
            }
        }
        Err(error) => {
            return Err(DirectoryError::UnableToAccessSource { error });
        }
    }

    if !source_directory_path.is_dir() {
        return Err(DirectoryError::SourceRootDirectoryIsNotADirectory);
    }


    match target_directory_path.try_exists() {
        Ok(exists) => {
            if exists && !options.allow_existing_target_directory {
                return Err(DirectoryError::TargetItemAlreadyExists);
            }
        }
        Err(error) => {
            return Err(DirectoryError::UnableToAccessSource { error });
        }
    }


    // Initialize a queue of file copy or directory create operations.
    let operation_queue = build_directory_copy_queue(
        source_directory_path,
        target_directory_path,
        options.maximum_copy_depth,
    )?;


    // So we've built the entire queue of operations, what's left is performing
    // the copy and directory create operations *precisely in the defined order*.
    // If we ignored the order, we could get into situations where
    // a directory didn't exist yet, but we would want to copy a file into it.

    let mut total_bytes_copied = 0;
    let mut num_files_copied = 0;
    let mut num_directories_created = 0;

    // TODO feature flag to use fs-err?

    for operation in operation_queue {
        match operation {
            QueuedOperation::CopyFile {
                source_path,
                source_size_bytes,
                target_path,
            } => {
                if target_path.exists() {
                    return Err(DirectoryError::TargetItemAlreadyExists);
                }

                copy_file(
                    source_path,
                    target_path,
                    FileCopyOptions {
                        overwrite_existing: options.overwrite_existing_files,
                        skip_existing: false,
                    },
                )
                .map_err(|error| match error {
                    FileError::NotFound => DirectoryError::SourceItemNotFound,
                    FileError::NotAFile => DirectoryError::SourceItemNotFound,
                    FileError::UnableToAccessSourceFile { error } => {
                        DirectoryError::UnableToAccessSource { error }
                    }
                    FileError::AlreadyExists => {
                        DirectoryError::TargetItemAlreadyExists
                    }
                    FileError::UnableToAccessTargetFile { error } => {
                        DirectoryError::UnableToAccessTarget { error }
                    }
                    FileError::SourceAndTargetAreTheSameFile => {
                        DirectoryError::SourceAndTargetAreTheSame
                    }
                    FileError::OtherIoError { error } => {
                        DirectoryError::OtherIoError { error }
                    }
                })?;

                num_files_copied += 1;
                total_bytes_copied += source_size_bytes;
            }
            QueuedOperation::CreateDirectory {
                target_directory_path,
            } => {
                std::fs::create_dir(target_directory_path).map_err(|error| {
                    DirectoryError::UnableToAccessTarget { error }
                })?;

                num_directories_created += 1;
            }
        };
    }

    Ok(FinishedDirectoryCopy {
        total_bytes_copied,
        num_files_copied,
        num_directories_created,
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

        let rejoin_result =
            rejoin_source_subpath_onto_target(root_a, foo, root_b);

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
