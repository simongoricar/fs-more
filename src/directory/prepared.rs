use std::{
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use_enabled_fs_module!();


use super::{
    common::DestinationDirectoryRule,
    is_directory_empty_unchecked,
    BrokenSymlinkBehaviour,
    DirectoryCopyDepthLimit,
    SymlinkBehaviour,
};
use crate::{
    directory::common::join_relative_source_path_onto_destination,
    error::{
        CopyDirectoryPreparationError,
        DestinationDirectoryPathValidationError,
        DirectoryExecutionPlanError,
        SourceDirectoryPathValidationError,
    },
};


#[cfg(windows)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum SymlinkType {
    File,
    Directory,
}


/// Represents a queued file copy, symlink, or directory creation operation.
///
/// For more details, see the [`build_directory_copy_queue`] function.
#[derive(Clone, Debug)]
pub(crate) enum QueuedOperation {
    /// Copy a file from `source_file_path` to `destination_file_path`.
    CopyFile {
        /// Where to copy a file from.
        source_file_path: PathBuf,

        /// Where to copy a file to.
        destination_file_path: PathBuf,

        /// Size of the `source_file_path` file in bytes.
        source_size_bytes: u64,
    },

    /// Create a directory at `destination_directory_path`.
    CreateDirectory {
        /// Directory to create.
        destination_directory_path: PathBuf,

        /// Whether to automatically create missing parent directories as well.
        ///
        /// This will use [`fs::create_dir_all`] instead of [`fs::create_dir`].
        create_parent_directories: bool,

        /// Size of the `destination_directory_path` directory in bytes.
        /// This is the size of the directory "file" itself on the filesystem,
        /// not a recursive size scan.
        source_size_bytes: u64,
    },

    /// Create a symbolic link at `symlink_path`.
    CreateSymlink {
        /// Where to create the symbolic link.
        ///
        /// This is a path under the destination directory, but this is not reflected in the
        /// field name to avoid mixups with meaning of "destination".
        symlink_path: PathBuf,

        /// This specifies whether the symlink destination is a file or a directory.
        ///
        /// This is present only on Windows targets, as we need to use a different API
        /// according to the symlink destination type.
        #[cfg(windows)]
        symlink_destination_type: SymlinkType,

        /// Where the symbolink link should point to.
        symlink_destination_path: PathBuf,

        /// Size of the symbolic link we're "copying".
        source_symlink_size_bytes: u64,
    },
}


/// Returns a boolean indicating whether the provided path exists.
///
/// Behaves similarly to [`fs::try_exists`],
/// but *does not follow symbolic links*.
///
/// If the `fs-err` feature flag is enabled, this function will automatically use it.
pub(crate) fn try_exists_without_follow(path: &Path) -> std::io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) => match error.kind() {
            ErrorKind::NotFound => Ok(false),
            _ => Err(error),
        },
    }
}



/// Information about a validated source path (used in copying and moving directories).
#[derive(Clone, Debug)]
pub(crate) struct ValidatedSourceDirectory {
    pub(crate) directory_path: PathBuf,
    pub(crate) unfollowed_directory_path: PathBuf,
    pub(crate) original_path_was_symlink_to_directory: bool,
}

/// Ensures the given source directory path is valid.
///
/// This means that it exists, and that it is a directory.
/// Failing to find out whether it exists, or any similar read restriction,
/// will result in an error as well.
///
/// The returned [`ValidatedSourceDirectory`] contains the canonical path
/// of the provided `source_directory_path` which you should use in the future,
/// and, most importantly, for any path comparisons.
///
/// This also means that if `source_directory_path` is a symbolic link to a directory,
/// the validated version will have followed the link to its destination.
pub(super) fn validate_source_directory_path(
    source_directory_path: &Path,
) -> Result<ValidatedSourceDirectory, SourceDirectoryPathValidationError> {
    // Ensure the source directory path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `DirectoryError::NotFound` error.
    match try_exists_without_follow(source_directory_path) {
        Ok(exists) => {
            if !exists {
                return Err(SourceDirectoryPathValidationError::NotFound {
                    directory_path: source_directory_path.to_path_buf(),
                });
            }
        }
        Err(error) => {
            return Err(SourceDirectoryPathValidationError::UnableToAccess {
                directory_path: source_directory_path.to_path_buf(),
                error,
            });
        }
    }


    let is_symlink_to_directory = {
        let metadata_without_follow =
            fs::symlink_metadata(source_directory_path).map_err(|error| {
                SourceDirectoryPathValidationError::UnableToAccess {
                    directory_path: source_directory_path.to_path_buf(),
                    error,
                }
            })?;

        if metadata_without_follow.is_symlink() {
            let metadata_with_follow = fs::metadata(source_directory_path).map_err(|error| {
                SourceDirectoryPathValidationError::UnableToAccess {
                    directory_path: source_directory_path.to_path_buf(),
                    error,
                }
            })?;

            if !metadata_with_follow.is_dir() {
                return Err(SourceDirectoryPathValidationError::NotADirectory {
                    path: source_directory_path.to_path_buf(),
                });
            } else {
                true
            }
        } else if metadata_without_follow.is_dir() {
            false
        } else {
            return Err(SourceDirectoryPathValidationError::NotADirectory {
                path: source_directory_path.to_path_buf(),
            });
        }
    };


    let canonical_source_directory_path =
        fs::canonicalize(source_directory_path).map_err(|error| {
            SourceDirectoryPathValidationError::UnableToAccess {
                directory_path: source_directory_path.to_path_buf(),
                error,
            }
        })?;


    #[cfg(feature = "dunce")]
    {
        let de_unced_canonical_path =
            dunce::simplified(&canonical_source_directory_path).to_path_buf();

        Ok(ValidatedSourceDirectory {
            directory_path: de_unced_canonical_path,
            unfollowed_directory_path: source_directory_path.to_path_buf(),
            original_path_was_symlink_to_directory: is_symlink_to_directory,
        })
    }

    #[cfg(not(feature = "dunce"))]
    {
        Ok(ValidatedSourceDirectory {
            directory_path: canonical_source_directory_path,
            unfollowed_directory_path: source_directory_path.to_path_buf(),
            original_path_was_symlink_to_directory: is_symlink_to_directory,
        })
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DestinationDirectoryState {
    /// Destination directory does not exist.
    DoesNotExist,

    /// Destination directory exists, but is empty.
    IsEmpty,

    /// Destination directory exists, and is not empty.
    IsNotEmpty,
}

impl DestinationDirectoryState {
    pub(crate) fn exists(&self) -> bool {
        !matches!(self, Self::DoesNotExist)
    }
}


/// Information about a validated target path (used in copying and moving directories).
///
/// "Valid" in this context means that it respects the user-provided `options`,
/// see [`validate_target_directory_path`].
#[derive(Clone, Debug)]
pub(crate) struct ValidatedDestinationDirectory {
    pub(crate) directory_path: PathBuf,
    pub(crate) state: DestinationDirectoryState,
}

/// Ensures the given destination directory path is valid.
///
/// This means that it respects the provided [`DestinationDirectoryRule`].
///
/// The returned [`ValidatedDestinationDirectory`] contains the canonical path
/// of the provided `destination_directory_path` which you should use in the future,
/// and, most importantly, for any path comparisons.
///
/// This also means that if `destination_directory_path` exists, and is a symbolic link to a directory,
/// the validated version will have followed the link to its target.
pub(super) fn validate_destination_directory_path(
    destination_directory_path: &Path,
    destination_directory_rule: DestinationDirectoryRule,
) -> Result<ValidatedDestinationDirectory, DestinationDirectoryPathValidationError> {
    let destination_directory_exists = try_exists_without_follow(destination_directory_path)
        .map_err(|error| DestinationDirectoryPathValidationError::UnableToAccess {
            directory_path: destination_directory_path.to_path_buf(),
            error,
        })?;

    // If `destination_directory_path` exists, but does not point to a directory,
    // we should abort.
    if destination_directory_exists && !destination_directory_path.is_dir() {
        return Err(DestinationDirectoryPathValidationError::NotADirectory {
            directory_path: destination_directory_path.to_path_buf(),
        });
    }


    let resolved_destination_directory_path = if destination_directory_exists {
        let canonical_destination_directory_path = fs::canonicalize(destination_directory_path)
            .map_err(|error| DestinationDirectoryPathValidationError::UnableToAccess {
                directory_path: destination_directory_path.to_path_buf(),
                error,
            })?;


        #[cfg(feature = "dunce")]
        {
            dunce::simplified(&canonical_destination_directory_path).to_path_buf()
        }

        #[cfg(not(feature = "dunce"))]
        {
            canonical_destination_directory_path
        }
    } else {
        destination_directory_path.to_path_buf()
    };


    let destination_directory_state = if destination_directory_exists {
        let is_empty = is_directory_empty_unchecked(&resolved_destination_directory_path).map_err(
            |error| DestinationDirectoryPathValidationError::UnableToAccess {
                directory_path: resolved_destination_directory_path.to_path_buf(),
                error,
            },
        )?;

        if is_empty {
            DestinationDirectoryState::IsEmpty
        } else {
            DestinationDirectoryState::IsNotEmpty
        }
    } else {
        DestinationDirectoryState::DoesNotExist
    };


    match destination_directory_rule {
        DestinationDirectoryRule::DisallowExisting => {
            if !matches!(destination_directory_state, DestinationDirectoryState::DoesNotExist) {
                return Err(DestinationDirectoryPathValidationError::AlreadyExists {
                    path: resolved_destination_directory_path,
                    destination_directory_rule,
                });
            }
        }
        DestinationDirectoryRule::AllowEmpty => {
            if !matches!(
                destination_directory_state,
                DestinationDirectoryState::DoesNotExist | DestinationDirectoryState::IsEmpty
            ) {
                return Err(DestinationDirectoryPathValidationError::NotEmpty {
                    directory_path: resolved_destination_directory_path,
                    destination_directory_rule,
                });
            }
        }
        DestinationDirectoryRule::AllowNonEmpty { .. } => {}
    }


    Ok(ValidatedDestinationDirectory {
        directory_path: resolved_destination_directory_path,
        state: destination_directory_state,
    })
}



/// Given a source and destination directory path, intended for copying or moving,
/// this function ensures the provided path *pair* is valid.
///
/// **Both paths MUST already be their canonical versions**,
/// for example, outputs of [`validate_source_directory_path`]
/// and [`validate_destination_directory_path`].
///
/// This fails, for example, when `source_directory_path` is a sub-path
/// of `destination_directory_path`.
pub(super) fn validate_source_destination_directory_pair(
    source_directory_path: &Path,
    destination_directory_path: &Path,
) -> Result<(), DestinationDirectoryPathValidationError> {
    // Ensure `destination_directory_path` isn't equal,
    // or a subdirectory of, `source_directory_path`˙.
    if destination_directory_path.starts_with(source_directory_path) {
        return Err(DestinationDirectoryPathValidationError::DescendantOfSourceDirectory {
            destination_directory_path: destination_directory_path.to_path_buf(),
            source_directory_path: source_directory_path.to_path_buf(),
        });
    }

    Ok(())
}


/// Given a source and destination directory as well as the maximum copy depth,
/// this function builds a list of [`QueuedOperation`]s that are needed to fully,
/// or up to the depth limit, copy the source directory to the destination directory.
///
/// The order of directory creation and file copying operations is such that
/// for any file in the list, the creation of its parent directory
/// appears before it in the queue.
///
/// Note, however, that **the queued operations do not include creation of
/// the `destination_directory_path` directory itself**,
/// even if that is necessary for a copy; it is up to the consumer to create
/// `destination_directory_path`, if need be, before executing the queue.
fn scan_and_plan_directory_copy(
    validated_source_directory: &ValidatedSourceDirectory,
    validated_destination_directory: &ValidatedDestinationDirectory,
    copy_depth_limit: DirectoryCopyDepthLimit,
    symlink_behaviour: SymlinkBehaviour,
    broken_symlink_behaviour: BrokenSymlinkBehaviour,
) -> Result<Vec<QueuedOperation>, DirectoryExecutionPlanError> {
    let mut operation_queue: Vec<QueuedOperation> = Vec::new();


    // Special case: if the source directory path was a symbolic link to a directory
    // and the symlink behaviour is set to keep, we should preserve that symlink on the destination.
    // This means we only need one operation.
    if symlink_behaviour == SymlinkBehaviour::Keep
        && validated_source_directory.original_path_was_symlink_to_directory
    {
        let source_symlink_size_bytes =
            fs::symlink_metadata(&validated_source_directory.directory_path)
                .map_err(|error| DirectoryExecutionPlanError::UnableToAccess {
                    path: validated_source_directory.directory_path.clone(),
                    error,
                })?
                .len();


        #[cfg(windows)]
        {
            operation_queue.push(QueuedOperation::CreateSymlink {
                symlink_path: validated_destination_directory.directory_path.to_path_buf(),
                symlink_destination_type: SymlinkType::Directory,
                source_symlink_size_bytes,
                symlink_destination_path: validated_source_directory.directory_path.to_path_buf(),
            });
        }

        #[cfg(not(windows))]
        {
            operation_queue.push(QueuedOperation::CreateSymlink {
                symlink_path: validated_destination_directory.directory_path.to_path_buf(),
                source_symlink_size_bytes,
                symlink_destination_path: validated_source_directory.directory_path.to_path_buf(),
            });
        }


        return Ok(operation_queue);
    }


    // Queue creating the base destination directory if needed.
    if !validated_destination_directory.state.exists() {
        let source_path_size_bytes =
            fs::symlink_metadata(&validated_source_directory.directory_path)
                .map_err(|error| DirectoryExecutionPlanError::UnableToAccess {
                    path: validated_source_directory.directory_path.to_path_buf(),
                    error,
                })?
                .len();

        operation_queue.push(QueuedOperation::CreateDirectory {
            source_size_bytes: source_path_size_bytes,
            destination_directory_path: validated_destination_directory
                .directory_path
                .to_path_buf(),
            create_parent_directories: true,
        });
    }


    // Scan the source directory and queue all copy and
    // directory creation operations that need to happen.
    struct PendingDirectoryScan {
        directory_path: PathBuf,
        directory_path_without_symlink_follows: PathBuf,
        depth: usize,
    }

    let mut directory_scan_queue = Vec::new();
    directory_scan_queue.push(PendingDirectoryScan {
        directory_path: validated_source_directory.directory_path.clone(),
        directory_path_without_symlink_follows: validated_source_directory.directory_path.clone(),
        depth: 0,
    });


    // TODO Refactor this giant loop into smaller functions.

    while let Some(next_directory) = directory_scan_queue.pop() {
        // Scan the directory for its files and directories.
        // Files are queued for copying, directories are queued for creation.
        let directory_iterator = fs::read_dir(&next_directory.directory_path).map_err(|error| {
            DirectoryExecutionPlanError::UnableToAccess {
                path: next_directory.directory_path.clone(),
                error,
            }
        })?;

        for directory_item in directory_iterator {
            let directory_item =
                directory_item.map_err(|error| DirectoryExecutionPlanError::UnableToAccess {
                    path: next_directory.directory_path.clone(),
                    error,
                })?;

            let directory_item_source_path = directory_item.path();
            let directory_item_name = directory_item_source_path.file_name().ok_or_else(|| {
                DirectoryExecutionPlanError::UnableToAccess {
                    path: directory_item_source_path.clone(),
                    error: io::Error::new(
                        ErrorKind::Other,
                        "ReadDir's iterator generated a path that terminates in \"..\"",
                    ),
                }
            })?;

            // We construct an updated `directory_path_without_symlink_follows` manually,
            // since following symlinks can make it hard to understand where under the
            // original source directory structure we are. But only if we have a sub-path of the
            // source directory can we correctly remap the relative sub-path inside the source
            // onto the destination directory.
            let new_directory_path_without_symlink_follows = next_directory
                .directory_path_without_symlink_follows
                .join(directory_item_name);


            // Remaps `new_directory_path_without_symlink_follows` (relative to `next_directory.source_directory_path`)
            // onto `destination_directory_path`, preserving directory structure.
            let directory_item_destination_path = join_relative_source_path_onto_destination(
                &validated_source_directory.directory_path,
                &new_directory_path_without_symlink_follows,
                &validated_destination_directory.directory_path,
            )
            .map_err(|error| {
                DirectoryExecutionPlanError::EntryEscapesSourceDirectory { path: error.path }
            })?;


            // For clarity: this call will not traverse symlinks.
            let item_type = directory_item.file_type().map_err(|error| {
                DirectoryExecutionPlanError::UnableToAccess {
                    path: directory_item_source_path.clone(),
                    error,
                }
            })?;


            if item_type.is_file() {
                let file_metadata = directory_item.metadata().map_err(|error| {
                    DirectoryExecutionPlanError::UnableToAccess {
                        path: directory_item_source_path.clone(),
                        error,
                    }
                })?;

                let file_size_in_bytes = file_metadata.len();


                operation_queue.push(QueuedOperation::CopyFile {
                    source_file_path: directory_item_source_path,
                    source_size_bytes: file_size_in_bytes,
                    destination_file_path: directory_item_destination_path,
                });
            } else if item_type.is_dir() {
                let directory_metadata = directory_item.metadata().map_err(|error| {
                    DirectoryExecutionPlanError::UnableToAccess {
                        path: directory_item_source_path.clone(),
                        error,
                    }
                })?;

                // Note that this is the size of the directory itself, not of its contents.
                let directory_size_in_bytes = directory_metadata.len();


                operation_queue.push(QueuedOperation::CreateDirectory {
                    source_size_bytes: directory_size_in_bytes,
                    destination_directory_path: directory_item_destination_path,
                    create_parent_directories: false,
                });


                // If we haven't reached the maximum depth yet, we queue the directory
                // to be scanned for further files and sub-directories.
                match copy_depth_limit {
                    DirectoryCopyDepthLimit::Limited { maximum_depth } => {
                        if next_directory.depth < maximum_depth {
                            directory_scan_queue.push(PendingDirectoryScan {
                                directory_path: directory_item_source_path.clone(),
                                directory_path_without_symlink_follows:
                                    new_directory_path_without_symlink_follows,
                                depth: next_directory.depth + 1,
                            });
                        }
                    }
                    DirectoryCopyDepthLimit::Unlimited => {
                        directory_scan_queue.push(PendingDirectoryScan {
                            directory_path: directory_item_source_path.clone(),
                            directory_path_without_symlink_follows:
                                new_directory_path_without_symlink_follows,
                            depth: next_directory.depth + 1,
                        });
                    }
                };
            } else if item_type.is_symlink() {
                // If the path is a symbolic link, we need to follow it and queue a copy
                // from the underlying file or directory.

                let resolved_symlink_path =
                    fs::read_link(&directory_item_source_path).map_err(|error| {
                        DirectoryExecutionPlanError::UnableToAccess {
                            path: directory_item_source_path.clone(),
                            error,
                        }
                    })?;


                let resolved_symlink_path_exists =
                    try_exists_without_follow(&resolved_symlink_path).map_err(|error| {
                        DirectoryExecutionPlanError::UnableToAccess {
                            path: resolved_symlink_path.clone(),
                            error,
                        }
                    })?;


                if !resolved_symlink_path_exists {
                    // This symbolic link is broken, we should look at the
                    // corresponding `broken_symlink_behaviour` option and act accordingly.

                    let unresolved_symlink_metadata =
                        fs::symlink_metadata(&directory_item_source_path).map_err(|error| {
                            DirectoryExecutionPlanError::UnableToAccess {
                                path: directory_item_source_path.clone(),
                                error,
                            }
                        })?;

                    let unresolved_symlink_file_size = unresolved_symlink_metadata.len();


                    #[cfg(windows)]
                    {
                        use std::os::windows::fs::FileTypeExt;

                        match broken_symlink_behaviour {
                            BrokenSymlinkBehaviour::Keep => {
                                let unresolved_symlink_file_type =
                                    unresolved_symlink_metadata.file_type();

                                let symbolic_link_type = if unresolved_symlink_file_type
                                    .is_symlink_file()
                                {
                                    SymlinkType::File
                                } else if unresolved_symlink_file_type.is_symlink_dir() {
                                    SymlinkType::Directory
                                } else {
                                    panic!("Unexpected symbolic link type: neither file nor directory.");
                                };


                                operation_queue.push(QueuedOperation::CreateSymlink {
                                    symlink_path: directory_item_destination_path,
                                    symlink_destination_type: symbolic_link_type,
                                    source_symlink_size_bytes: unresolved_symlink_file_size,
                                    symlink_destination_path: resolved_symlink_path,
                                });
                            }
                            BrokenSymlinkBehaviour::Abort => {
                                return Err(DirectoryExecutionPlanError::SymbolicLinkIsBroken {
                                    path: directory_item_source_path.clone(),
                                });
                            }
                        }
                    }

                    #[cfg(unix)]
                    {
                        match broken_symlink_behaviour {
                            BrokenSymlinkBehaviour::Keep => {
                                operation_queue.push(QueuedOperation::CreateSymlink {
                                    symlink_path: directory_item_destination_path,
                                    source_symlink_size_bytes: unresolved_symlink_file_size,
                                    symlink_destination_path: resolved_symlink_path,
                                });
                            }
                            BrokenSymlinkBehaviour::Abort => {
                                return Err(DirectoryExecutionPlanError::SymbolicLinkIsBroken {
                                    path: directory_item_source_path.clone(),
                                });
                            }
                        }
                    }

                    #[cfg(not(any(windows, unix)))]
                    {
                        compile_error!(
                            "fs-more supports only the following values of target_family: \
                            unix and windows (notably, wasm is unsupported)."
                        );
                    }

                    continue;
                }


                // Symbolic link is valid, we should look at the corresponding
                // `symlink_behaviour` option.

                let resolved_symlink_metadata =
                    fs::metadata(&resolved_symlink_path).map_err(|error| {
                        DirectoryExecutionPlanError::UnableToAccess {
                            path: resolved_symlink_path.clone(),
                            error,
                        }
                    })?;

                let resolved_symlink_file_type = resolved_symlink_metadata.file_type();
                let resolved_symlink_file_size = resolved_symlink_metadata.len();


                match symlink_behaviour {
                    SymlinkBehaviour::Keep => {
                        // Symbolic link should be preserved.
                        // Note that success is not guaranteed, e.g. in cases of
                        // trying to create symbolic links across mount points.

                        #[cfg(windows)]
                        {
                            let symlink_type = if resolved_symlink_file_type.is_file() {
                                SymlinkType::File
                            } else if resolved_symlink_file_type.is_dir() {
                                SymlinkType::Directory
                            } else if resolved_symlink_file_type.is_symlink() {
                                // FIXME Can this branch ever be reached? Is panicking okay?
                                //       Context for future readers: this branch seems impossible to reach,
                                //       since we used fs::metadata, which follows symbolic links.
                                panic!(
                                    "unexpected filesystem state: followed symbolic link(s), \
                                    but arrived at another symbolic link"
                                )
                            } else {
                                // FIXME Can this branch ever be reached? Is panicking okay?
                                //       Context for future readers: this branch seems impossible to reach,
                                //       since we used fs::metadata. For this to happen, is_file, is_dir and is_symlink
                                //       all need to return `false`. If you encounter this panic, report it to the issue tracker.
                                panic!(
                                    "unexpected filesystem state: followed symbolic link(s), \
                                    but arrived at something that is none of: file, directory, symlink"
                                );
                            };


                            operation_queue.push(QueuedOperation::CreateSymlink {
                                symlink_path: directory_item_destination_path,
                                symlink_destination_type: symlink_type,
                                source_symlink_size_bytes: resolved_symlink_file_size,
                                symlink_destination_path: resolved_symlink_path,
                            });
                        }

                        #[cfg(unix)]
                        {
                            operation_queue.push(QueuedOperation::CreateSymlink {
                                symlink_path: directory_item_destination_path,
                                source_symlink_size_bytes: resolved_symlink_file_size,
                                symlink_destination_path: resolved_symlink_path,
                            });
                        }

                        #[cfg(not(any(windows, unix)))]
                        {
                            compile_error!(
                                "fs-more supports only the following values of target_family: \
                                unix and windows (notably, wasm is unsupported)."
                            );
                        }
                    }
                    SymlinkBehaviour::Follow => {
                        // Symbolic link should be resolved, and a copy of the
                        // symlink's destination to the copy destination should be queued.
                        if resolved_symlink_file_type.is_file() {
                            operation_queue.push(QueuedOperation::CopyFile {
                                source_file_path: resolved_symlink_path,
                                source_size_bytes: resolved_symlink_file_size,
                                destination_file_path: directory_item_destination_path,
                            });
                        } else if resolved_symlink_file_type.is_dir() {
                            operation_queue.push(QueuedOperation::CreateDirectory {
                                source_size_bytes: resolved_symlink_file_size,
                                destination_directory_path: directory_item_destination_path,
                                create_parent_directories: false,
                            });


                            // If we haven't reached the maximum depth yet,
                            // we queue the symlink-followed directory for scanning.
                            match copy_depth_limit {
                                DirectoryCopyDepthLimit::Limited { maximum_depth } => {
                                    if next_directory.depth < maximum_depth {
                                        directory_scan_queue.push(PendingDirectoryScan {
                                            directory_path: directory_item_source_path.clone(),
                                            directory_path_without_symlink_follows:
                                                new_directory_path_without_symlink_follows,
                                            depth: next_directory.depth + 1,
                                        });
                                    }
                                }
                                DirectoryCopyDepthLimit::Unlimited => {
                                    directory_scan_queue.push(PendingDirectoryScan {
                                        directory_path: directory_item_source_path,
                                        directory_path_without_symlink_follows:
                                            new_directory_path_without_symlink_follows,
                                        depth: next_directory.depth + 1,
                                    });
                                }
                            };
                        } else if resolved_symlink_file_type.is_symlink() {
                            // FIXME Can this branch ever be reached? Is panicking okay?
                            //       Context for future readers: this branch seems impossible to reach,
                            //       since we used fs::metadata, which follows symbolic links.
                            panic!(
                                "unexpected filesystem state: followed symbolic link(s), \
                                but arrived at another symbolic link"
                            )
                        }
                    }
                }
            }
        }
    }

    Ok(operation_queue)
}


/// Given a list of references to [`QueuedOperation`]s, this function validates that
/// the files and directories this queue would process match the provided [`DestinationDirectoryRule`].
fn check_operation_queue_for_collisions(
    queue: &[QueuedOperation],
    destination_directory_rules: DestinationDirectoryRule,
) -> Result<(), DirectoryExecutionPlanError> {
    let overwriting_existing_destination_files_allowed =
        destination_directory_rules.allows_overwriting_existing_destination_files();

    let existing_destination_subdirectories_allowed =
        destination_directory_rules.allows_existing_destination_subdirectories();


    if overwriting_existing_destination_files_allowed && existing_destination_subdirectories_allowed
    {
        // There is nothing to check, as we can have any collisions we want
        // if we allow everything to be overwritten.
        return Ok(());
    }


    for queue_item in queue {
        match queue_item {
            QueuedOperation::CopyFile {
                destination_file_path,
                ..
            } => {
                if !overwriting_existing_destination_files_allowed {
                    let destination_file_exists = try_exists_without_follow(destination_file_path)
                        .map_err(|error| DirectoryExecutionPlanError::UnableToAccess {
                            path: destination_file_path.to_path_buf(),
                            error,
                        })?;

                    if destination_file_exists {
                        return Err(DirectoryExecutionPlanError::DestinationItemAlreadyExists {
                            path: destination_file_path.clone(),
                        });
                    }
                }
            }

            QueuedOperation::CreateDirectory {
                destination_directory_path,
                ..
            } => {
                if !existing_destination_subdirectories_allowed {
                    let destination_directory_exists =
                        try_exists_without_follow(destination_directory_path).map_err(|error| {
                            DirectoryExecutionPlanError::UnableToAccess {
                                path: destination_directory_path.to_path_buf(),
                                error,
                            }
                        })?;

                    if destination_directory_exists {
                        return Err(DirectoryExecutionPlanError::DestinationItemAlreadyExists {
                            path: destination_directory_path.clone(),
                        });
                    }
                }
            }

            QueuedOperation::CreateSymlink { symlink_path, .. } => {
                if !overwriting_existing_destination_files_allowed {
                    let symlink_path_exists =
                        try_exists_without_follow(symlink_path).map_err(|error| {
                            DirectoryExecutionPlanError::UnableToAccess {
                                path: symlink_path.to_path_buf(),
                                error,
                            }
                        })?;

                    if symlink_path_exists {
                        return Err(DirectoryExecutionPlanError::DestinationItemAlreadyExists {
                            path: symlink_path.to_path_buf(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}



/// An auxiliary struct that contains a set of operations required for a directory copy.
///
/// It can be initialized by calling [`Self::prepare`] or [`Self::prepare_with_validated`].
pub(crate) struct DirectoryCopyPrepared {
    /// An array of ordered file copy and directory creation operations
    /// that togeher form a requested directory copy.
    pub(crate) operation_queue: Vec<QueuedOperation>,

    /// How many bytes will need to be copied (i.e. the source directory size).
    pub(crate) total_bytes: u64,
}


impl DirectoryCopyPrepared {
    /// Prepare for a new directory copy.
    ///
    /// This includes validating both the source and destination directory paths
    /// as well as preparing the operation queue.
    pub fn prepare(
        source_directory_path: &Path,
        destination_directory_path: &Path,
        destination_directory_rule: DestinationDirectoryRule,
        copy_depth_limit: DirectoryCopyDepthLimit,
        symlink_behaviour: SymlinkBehaviour,
        broken_symlink_behaviour: BrokenSymlinkBehaviour,
    ) -> Result<Self, CopyDirectoryPreparationError> {
        let (canonical_source_directory_path, validated_destination) =
            Self::validate_source_and_destination(
                source_directory_path,
                destination_directory_path,
                destination_directory_rule,
            )?;

        Self::prepare_with_validated(
            canonical_source_directory_path,
            validated_destination,
            destination_directory_rule,
            copy_depth_limit,
            symlink_behaviour,
            broken_symlink_behaviour,
        )
        .map_err(CopyDirectoryPreparationError::CopyPlanningError)
    }

    /// Prepare for a new directory copy with already-validated source and destination.
    ///
    /// This preparation therefore only includes preparing the operation queue.
    pub fn prepare_with_validated(
        validated_source_directory: ValidatedSourceDirectory,
        validated_destination_directory: ValidatedDestinationDirectory,
        destination_directory_rule: DestinationDirectoryRule,
        copy_depth_limit: DirectoryCopyDepthLimit,
        symlink_behaviour: SymlinkBehaviour,
        broken_symlink_behaviour: BrokenSymlinkBehaviour,
    ) -> Result<Self, DirectoryExecutionPlanError> {
        let operations = Self::prepare_directory_operations(
            &validated_source_directory,
            &validated_destination_directory,
            destination_directory_rule,
            copy_depth_limit,
            symlink_behaviour,
            broken_symlink_behaviour,
        )?;

        let bytes_total = Self::calculate_total_bytes_to_be_copied(&operations);


        Ok(Self {
            operation_queue: operations,
            total_bytes: bytes_total,
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
                QueuedOperation::CreateSymlink {
                    source_symlink_size_bytes: source_size_bytes,
                    ..
                } => *source_size_bytes,
            })
            .sum::<u64>()
    }

    fn validate_source_and_destination(
        source_directory_path: &Path,
        destination_directory_path: &Path,
        destination_directory_rule: DestinationDirectoryRule,
    ) -> Result<
        (ValidatedSourceDirectory, ValidatedDestinationDirectory),
        CopyDirectoryPreparationError,
    > {
        let validated_source_directory = validate_source_directory_path(source_directory_path)?;
        let validated_target_directory = validate_destination_directory_path(
            destination_directory_path,
            destination_directory_rule,
        )?;

        validate_source_destination_directory_pair(
            &validated_source_directory.directory_path,
            &validated_target_directory.directory_path,
        )?;

        Ok((validated_source_directory, validated_target_directory))
    }

    /// Scans the source directory and prepares a plan (a set of operations)
    /// to copy the source directory to the destination directory, as configured.
    ///
    /// <br>
    ///
    /// We also do a destination directory collision check in the hopes of catching existing mismatches
    /// of the provided `destination_directory_rule` early, before we actually copy any file at all.
    /// This way the target directory stays intact if there is any collision,
    /// instead of returning an error after having copied some files already,
    /// which would leave the destination directory in an unpredictable state.
    ///
    /// It's of course still possible that the destination directory ends up in an unpredictable state,
    /// since a [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
    /// race condition is still possible.
    /// However, those cases should be very rare and are essentially unsolvable,
    /// unless there was a robust rollback mechanism (but this would require platform-specific implementation).
    /// For example: Windows
    /// [cautions against using transactional NTFS](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-findfirstfiletransacteda)).
    fn prepare_directory_operations(
        validated_source_directory: &ValidatedSourceDirectory,
        validated_destination_directory: &ValidatedDestinationDirectory,
        destination_directory_rule: DestinationDirectoryRule,
        copy_depth_limit: DirectoryCopyDepthLimit,
        symlink_behaviour: SymlinkBehaviour,
        broken_symlink_behaviour: BrokenSymlinkBehaviour,
    ) -> Result<Vec<QueuedOperation>, DirectoryExecutionPlanError> {
        // Initialize a queue of file copy or directory create operations.
        let copy_queue = scan_and_plan_directory_copy(
            validated_source_directory,
            validated_destination_directory,
            copy_depth_limit,
            symlink_behaviour,
            broken_symlink_behaviour,
        )?;

        check_operation_queue_for_collisions(&copy_queue, destination_directory_rule)?;

        Ok(copy_queue)
    }
}
