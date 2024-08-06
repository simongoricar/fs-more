use std::{
    collections::VecDeque,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

use super::{file::FileComparisonErrorInner, path_type::PathType};
use crate::assertable::file::{ensure_contents_of_files_are_equal_inner, FileComparisonOptions};



/// An internal error that can occur when comparing directories.
///
/// This error type is never returned by itself, see [`DirectoryComparisonError`]
/// that wraps this type with useful context.
#[derive(Debug, Error)]
enum DirectoryComparisonErrorInner {
    #[error(
        "unable to read directory: {}", .directory_path.display()
    )]
    UnableToReadDirectory {
        directory_path: PathBuf,

        #[source]
        error: std::io::Error,
    },

    #[error(
        "unable to read entry for directory: {}", .directory_path.display()
    )]
    UnableToReadDirectoryEntry {
        directory_path: PathBuf,

        #[source]
        error: std::io::Error,
    },

    #[error(
        "invalid directory entry: {}", .path.display()
    )]
    InvalidDirectoryEntry { path: PathBuf },

    #[error(
        "unable to access path: {}", .path.display()
    )]
    UnableToAccessPath {
        path: PathBuf,

        #[source]
        error: std::io::Error,
    },

    #[error(
        "directory contents do not match;\n  \
          {}\n\
        is a {} inside the secondary directory, \
        but the corresponding path inside the primary directory\n  \
          {}\n\
        is a {}.",
        .original_path.display(),
        .original_path_type.to_short_name(),
        .expected_path.display(),
        .expected_path_type.to_short_name()
    )]
    PrimaryPathIsOfIncorrectType {
        original_path: PathBuf,

        original_path_type: PathType,

        expected_path: PathBuf,

        expected_path_type: PathType,
    },

    #[error(
        "directory contents do not match;\n  \
          {}\n\
        and\n  \
          {}\n\
        do not match.",
        .original_path.display(),
        .expected_path.display()
    )]
    FileComparisonError {
        original_path: PathBuf,

        expected_path: PathBuf,

        #[source]
        error: FileComparisonErrorInner,
    },

    #[error(
        "broken symbolic links exist on both sides, \
        but their destinations do not match;\n  \
          \"{}\" (in primary) points to \"{}\"\n\
        and\n  \
          \"{}\" (in secondary) points to \"{}\".",
        .symlink_path_in_primary.display(),
        .resolved_symlink_path_in_primary.display(),
        .symlink_path_in_secondary.display(),
        .resolved_symlink_path_in_secondary.display()
    )]
    BrokenSymlinkDestinationsNotEqual {
        symlink_path_in_primary: PathBuf,

        resolved_symlink_path_in_primary: PathBuf,

        symlink_path_in_secondary: PathBuf,

        resolved_symlink_path_in_secondary: PathBuf,
    },
}


/// An error that can occur when comparing directories.
#[derive(Debug, Error)]
#[error(
    "failed while comparing directory\n  \
       comparing:\n    \
         primary = \"{}\"\n    \
         secondary = \"{}\"\n\
     \n  \
       reason:\n\
     {:?}",
     .primary_directory_path.display(),
     .secondary_directory_path.display(),
     .reason
)]
pub struct DirectoryComparisonError {
    /// Internal reason for the comparison error.
    reason: DirectoryComparisonErrorInner,

    /// The primary directory path being compared.
    primary_directory_path: PathBuf,

    /// The secondary directory path being compared.
    secondary_directory_path: PathBuf,
}



/// Directory comparison options.
#[derive(Clone, Debug)]
pub struct DirectoryComparisonOptions {
    /// If `true`, the comparison will require that
    /// symlinks to files or directories on one side must be
    /// symlinks on the other as well. The contents of symlink
    /// destinations will then be compared.
    ///
    /// If `false`, the comparison will not mind if a symlink to a file
    /// or directory on one side, is actually a direct file or directory
    /// on the other, as long as the contents match.
    pub strict_symlink_comparison: bool,
}


/// For more information, see documentation of
/// [`assert_primary_directory_precisely_contains_secondary_directory`].
#[allow(clippy::result_large_err)]
fn ensure_primary_directory_precisely_contains_secondary_directory_inner(
    primary_directory_path: &Path,
    secondary_directory_path: &Path,
    options: DirectoryComparisonOptions,
) -> Result<(), DirectoryComparisonErrorInner> {
    // Sets up a queue for depth-first scanning.

    struct PendingDirectory {
        /// `secondary_directory_path`, a subdirectory of it,
        /// or somewhere else entirely, when we follow e.g. a symlink to a directory.
        directory_path_to_scan: PathBuf,

        /// `primary_directory_path`, a subdirectory of it,
        /// or somewhere else entirely, when we follow e.g. a symlink to a directory.
        directory_path_to_compare_to: PathBuf,
    }

    let mut scan_queue = VecDeque::with_capacity(1);
    scan_queue.push_back(PendingDirectory {
        directory_path_to_scan: secondary_directory_path.to_path_buf(),
        directory_path_to_compare_to: primary_directory_path.to_path_buf(),
    });


    while let Some(pending_directory) = scan_queue.pop_front() {
        // After retrieving the next directory in the queue, we perform a scan of its contents
        // and attempt to compare the equivalent paths in the primary directory.

        let directory_scan =
            fs::read_dir(&pending_directory.directory_path_to_scan).map_err(|error| {
                DirectoryComparisonErrorInner::UnableToReadDirectory {
                    directory_path: pending_directory.directory_path_to_scan.to_path_buf(),
                    error,
                }
            })?;


        for entry in directory_scan {
            let entry = entry.map_err(|error| {
                DirectoryComparisonErrorInner::UnableToReadDirectoryEntry {
                    directory_path: pending_directory.directory_path_to_scan.to_path_buf(),
                    error,
                }
            })?;



            let entry_path = entry.path();
            let entry_file_name = entry_path.file_name().ok_or_else(|| {
                DirectoryComparisonErrorInner::InvalidDirectoryEntry {
                    path: entry_path.to_path_buf(),
                }
            })?;

            let entry_path_type = PathType::from_path(&entry_path).map_err(|error| {
                DirectoryComparisonErrorInner::UnableToAccessPath {
                    path: entry_path.clone(),
                    error,
                }
            })?;



            let remapped_onto_comparison_target = pending_directory
                .directory_path_to_compare_to
                .join(entry_file_name);

            let remapped_path_type = PathType::from_path(&remapped_onto_comparison_target)
                .map_err(|error| DirectoryComparisonErrorInner::UnableToAccessPath {
                    path: remapped_onto_comparison_target.clone(),
                    error,
                })?;



            match entry_path_type {
                PathType::NotFound => {
                    return Err(DirectoryComparisonErrorInner::InvalidDirectoryEntry {
                        path: entry_path,
                    });
                }

                PathType::BareFile | PathType::SymlinkToFile => {
                    // Scanned path is file.
                    // We now check if the path remapped into the primary directory exists.
                    // If it does, we must compare it byte by byte.

                    match remapped_onto_comparison_target.try_exists() {
                        Ok(exists) => {
                            if !exists {
                                return Err(
                                    DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                        original_path: entry_path.clone(),
                                        original_path_type: PathType::BareFile,
                                        expected_path: remapped_onto_comparison_target.clone(),
                                        expected_path_type: PathType::NotFound,
                                    },
                                );
                            }
                        }
                        Err(error) => {
                            return Err(DirectoryComparisonErrorInner::UnableToAccessPath {
                                path: remapped_onto_comparison_target,
                                error,
                            });
                        }
                    }


                    // Check contents of files byte by byte (this will also do path type checks).
                    let comparison_result = ensure_contents_of_files_are_equal_inner(
                        &entry_path,
                        &remapped_onto_comparison_target,
                        FileComparisonOptions {
                            strict_symlink_comparison: options.strict_symlink_comparison,
                        },
                    );

                    if let Err(comparison_error) = comparison_result {
                        return Err(DirectoryComparisonErrorInner::FileComparisonError {
                            original_path: entry_path,
                            expected_path: remapped_onto_comparison_target,
                            error: comparison_error,
                        });
                    }
                }

                PathType::BareDirectory => {
                    // Scanned path is a directory (and *not* a symlink to one).
                    // We now check if the path remapped into the primary directory exists.
                    // If it does, we must check its type as well.

                    match remapped_path_type {
                        PathType::NotFound => {
                            return Err(
                                DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                    original_path: entry_path.clone(),
                                    original_path_type: PathType::BareDirectory,
                                    expected_path: remapped_onto_comparison_target.clone(),
                                    expected_path_type: PathType::NotFound,
                                },
                            );
                        }
                        PathType::BareDirectory => {
                            // Scanned path is a non-symlink directory, and the remapped path
                            // inside the primary directory is also a non-symlink directory.
                            // This is good, so we queue the directories for further comparison.

                            scan_queue.push_front(PendingDirectory {
                                directory_path_to_scan: entry_path,
                                directory_path_to_compare_to: remapped_onto_comparison_target,
                            });
                        }
                        PathType::SymlinkToDirectory => {
                            // Scanned path is a non-symlink directory, and the remapped path
                            // inside the primary directory leads to a symlink to a directory.
                            // This is ok if `strict_symlink_comparison` is `false`, and errors otherwise.

                            if options.strict_symlink_comparison {
                                return Err(
                                    DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                        original_path: entry_path.clone(),
                                        original_path_type: PathType::BareDirectory,
                                        expected_path: remapped_onto_comparison_target.clone(),
                                        expected_path_type: PathType::SymlinkToDirectory,
                                    },
                                );
                            }

                            let resolved_remapped_symlink_path = fs::read_link(
                                &remapped_onto_comparison_target,
                            )
                            .map_err(|error| DirectoryComparisonErrorInner::UnableToAccessPath {
                                path: entry_path.clone(),
                                error,
                            })?;

                            scan_queue.push_front(PendingDirectory {
                                directory_path_to_scan: entry_path,
                                directory_path_to_compare_to: resolved_remapped_symlink_path,
                            });
                        }
                        other_path_type => {
                            // Scanned path is a non-symlink directory, and the remapped path
                            // inside the primary directory is neither a directory nor a symlink to one.
                            // This means the comparison fails.

                            return Err(
                                DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                    original_path: entry_path.clone(),
                                    original_path_type: PathType::BareDirectory,
                                    expected_path: remapped_onto_comparison_target.clone(),
                                    expected_path_type: other_path_type,
                                },
                            );
                        }
                    }
                }

                PathType::SymlinkToDirectory => {
                    // Scanned path is a symlink to a directory.
                    // We now check if the path remapped into the primary directory exists.
                    // If it does, we ensure its type is ok as well.

                    let resolved_entry_path = fs::read_link(&entry_path).map_err(|error| {
                        DirectoryComparisonErrorInner::UnableToAccessPath {
                            path: entry_path.clone(),
                            error,
                        }
                    })?;


                    match remapped_path_type {
                        PathType::NotFound => {
                            return Err(
                                DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                    original_path: entry_path.clone(),
                                    original_path_type: entry_path_type,
                                    expected_path: remapped_onto_comparison_target.clone(),
                                    expected_path_type: remapped_path_type,
                                },
                            );
                        }
                        PathType::SymlinkToDirectory => {
                            // Both the scanned and the remapped path inside the primary
                            // directory is a symlink to a directory. We should now resolve
                            // the symlinks and queue them for comparison.

                            let resolved_remapped_symlink_path = fs::read_link(
                                &remapped_onto_comparison_target,
                            )
                            .map_err(|error| DirectoryComparisonErrorInner::UnableToAccessPath {
                                path: remapped_onto_comparison_target.clone(),
                                error,
                            })?;


                            scan_queue.push_front(PendingDirectory {
                                directory_path_to_scan: resolved_entry_path,
                                directory_path_to_compare_to: resolved_remapped_symlink_path,
                            });
                        }
                        PathType::BareDirectory => {
                            // The scanned path is a symlink to a directory, but the remapped path
                            // inside the primary directory is a normal directory.
                            // If `strict_symlink_comparison` is `false`, this is valid,
                            // otherwise this returns an errors.

                            if options.strict_symlink_comparison {
                                return Err(
                                    DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                        original_path: entry_path.clone(),
                                        original_path_type: entry_path_type,
                                        expected_path: remapped_onto_comparison_target.clone(),
                                        expected_path_type: remapped_path_type,
                                    },
                                );
                            }

                            scan_queue.push_front(PendingDirectory {
                                directory_path_to_scan: resolved_entry_path,
                                directory_path_to_compare_to: remapped_onto_comparison_target,
                            });
                        }
                        other_path_type => {
                            return Err(
                                DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                    original_path: entry_path.clone(),
                                    original_path_type: entry_path_type,
                                    expected_path: remapped_onto_comparison_target.clone(),
                                    expected_path_type: other_path_type,
                                },
                            );
                        }
                    }
                }

                PathType::BrokenSymlink => {
                    match remapped_path_type {
                        PathType::BrokenSymlink => {
                            // Both locations are broken symbolic links. The way we'll compare them
                            // is by checking if fs::read_link resolves to the same location.
                            let resolved_entry_path =
                                fs::read_link(&entry_path).map_err(|error| {
                                    DirectoryComparisonErrorInner::UnableToAccessPath {
                                        path: entry_path.clone(),
                                        error,
                                    }
                                })?;

                            let resolved_remapped_path = fs::read_link(
                                &remapped_onto_comparison_target,
                            )
                            .map_err(|error| DirectoryComparisonErrorInner::UnableToAccessPath {
                                path: remapped_onto_comparison_target.clone(),
                                error,
                            })?;


                            if resolved_entry_path != resolved_remapped_path {
                                return Err(DirectoryComparisonErrorInner::BrokenSymlinkDestinationsNotEqual {
                                    symlink_path_in_primary: remapped_onto_comparison_target,
                                    resolved_symlink_path_in_primary: resolved_remapped_path,
                                    symlink_path_in_secondary: entry_path,
                                    resolved_symlink_path_in_secondary: resolved_entry_path
                                });
                            }
                        }
                        unmatched_type => {
                            return Err(
                                DirectoryComparisonErrorInner::PrimaryPathIsOfIncorrectType {
                                    original_path: entry_path.clone(),
                                    original_path_type: entry_path_type,
                                    expected_path: remapped_onto_comparison_target.clone(),
                                    expected_path_type: unmatched_type,
                                },
                            );
                        }
                    }
                }

                PathType::Unrecognized => {
                    return Err(DirectoryComparisonErrorInner::InvalidDirectoryEntry {
                        path: entry_path,
                    });
                }
            }
        }
    }

    Ok(())
}



/// Given an [`Error`] trait object, this function formats the error
/// as a string, including writing out its [`Error::source`].
fn format_error_with_source(error: &dyn Error) -> String {
    let formatted_cause = match error.source() {
        Some(cause) => {
            let formatted_error = format_error_with_source(cause);

            // Creates a nice two-space indentation level for each nested error source.
            formatted_error.replace('\n', "\n  ")
        }
        None => "".to_string(),
    };

    format!("{}\n -> caused by: {}", error, formatted_cause)
}


/// Asserts that the `primary_directory_path` contains everything that `secondary_directory_path` has,
/// but not necessarily the other way around (`primary_directory_path` can have extra entries).
///
/// Every file and subdirectory in the `secondary_directory_path` is compared with the corresponding
/// file and subdirectory in `primary_directory_path`. File contents are fully compared,
/// erroring on content mismath.
///
/// For symlink behaviour, see [`DirectoryComparisonOptions`].
#[track_caller]
pub(crate) fn assert_primary_directory_precisely_contains_secondary_directory<F, S>(
    primary_directory_path: F,
    secondary_directory_path: S,
    options: DirectoryComparisonOptions,
) where
    F: AsRef<Path>,
    S: AsRef<Path>,
{
    let assertion_result = ensure_primary_directory_precisely_contains_secondary_directory_inner(
        primary_directory_path.as_ref(),
        secondary_directory_path.as_ref(),
        options,
    );

    if let Err(comparison_error) = assertion_result {
        panic!("{}", format_error_with_source(&comparison_error));
    }
}


/// Asserts that the `primary_directory_path` and `secondary_directory_path` fully match content-wise.
///
/// This means that, unlike with [`assert_primary_directory_precisely_contains_secondary_directory`],
/// neither `primary_directory_path` nor `secondary_directory_path` cannot have extra entries -- they
/// must be fully the same.
///
/// Every file and subdirectory in the `secondary_directory_path` is compared with the corresponding
/// file and subdirectory in `primary_directory_path` (and the other way around).
/// File contents are fully compared, erroring on content mismath.
///
/// For symlink behaviour, see [`DirectoryComparisonOptions`].
#[track_caller]
pub(crate) fn assert_primary_directory_fully_matches_secondary_directory<F, S>(
    primary_directory_path: F,
    secondary_directory_path: S,
    options: DirectoryComparisonOptions,
) where
    F: AsRef<Path>,
    S: AsRef<Path>,
{
    let assertion_result = ensure_primary_directory_precisely_contains_secondary_directory_inner(
        primary_directory_path.as_ref(),
        secondary_directory_path.as_ref(),
        options.clone(),
    );

    if let Err(comparison_error) = assertion_result {
        panic!("{}", format_error_with_source(&comparison_error));
    }


    let assertion_result = ensure_primary_directory_precisely_contains_secondary_directory_inner(
        secondary_directory_path.as_ref(),
        primary_directory_path.as_ref(),
        options,
    );

    if let Err(comparison_error) = assertion_result {
        panic!("{}", format_error_with_source(&comparison_error));
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::assertable::AsPath;
    use crate::trees::structures::deep::DeepTree;
    use crate::trees::FileSystemHarness;


    #[test]
    fn two_identical_directories_match() {
        let deep_tree = DeepTree::initialize();
        let deep_tree_copy = DeepTree::initialize();

        assert_primary_directory_fully_matches_secondary_directory(
            deep_tree.as_path(),
            deep_tree_copy.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );

        assert_primary_directory_fully_matches_secondary_directory(
            deep_tree_copy.as_path(),
            deep_tree.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );


        assert_primary_directory_precisely_contains_secondary_directory(
            deep_tree.as_path(),
            deep_tree_copy.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );

        assert_primary_directory_precisely_contains_secondary_directory(
            deep_tree_copy.as_path(),
            deep_tree.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );


        deep_tree.destroy();
        deep_tree_copy.destroy();
    }
}
