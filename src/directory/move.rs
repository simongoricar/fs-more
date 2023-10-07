use std::path::Path;

use super::{copy::TargetDirectoryRule, copy_directory_unchecked, DirectoryScan};
use crate::{
    directory::{
        copy::{
            validate_source_directory_path,
            validate_source_target_directory_pair,
            validate_target_directory_path,
        },
        rejoin_source_subpath_onto_target,
        DirectoryCopyOptions,
    },
    error::{DirectoryError, DirectoryScanError, DirectorySizeScanError},
};

/// Options that influence the [`move_directory`] function.
pub struct DirectoryMoveOptions {
    /// Specifies whether you allow the target directory to exist before moving
    /// and whether it must be empty or not.
    ///
    /// If you allow a non-empty target directory, you may also specify whether you allow
    /// target files or subdirectories to already exist (and be overwritten).
    ///
    /// See [`TargetDirectoryRule`] for more details and examples.
    pub target_directory_rule: TargetDirectoryRule,
}

/// Describes actions taken by the [`copy_directory`][crate::directory::copy_directory] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FinishedDirectoryMove {
    /// Total amount of bytes moved.
    pub total_bytes_moved: u64,

    /// Number of files moved.
    pub num_files_moved: usize,

    /// Number of directories moved (created).
    pub num_directories_moved: usize,
}

struct DirectoryContentDetails {
    pub(crate) total_bytes: u64,
    pub(crate) total_files: usize,
    pub(crate) total_directories: usize,
}

/// Scans the provided directory (no depth limit).
/// 
/// The returned struct contains information about the total number files and directories and the total directory size.
#[rustfmt::skip]
fn collect_source_directory_details(
    source_directory_path: &Path
) -> Result<DirectoryContentDetails, DirectoryError> {
    let scan = DirectoryScan::scan_with_options(source_directory_path, None, false)
        .map_err(|error| match error {
            DirectoryScanError::NotFound => 
                DirectoryError::SourceDirectoryNotFound,
            DirectoryScanError::NotADirectory => 
                DirectoryError::SourceDirectoryIsNotADirectory,
            DirectoryScanError::UnableToReadDirectory { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectoryScanError::UnableToReadDirectoryItem { error } => 
                DirectoryError::UnableToAccessSource { error },
        })?;

    let total_size_in_bytes = scan.total_size_in_bytes()
        .map_err(|error| match error {
            DirectorySizeScanError::RootDirectoryNotFound => 
                DirectoryError::SourceDirectoryNotFound,
            DirectorySizeScanError::RootIsNotADirectory => 
                DirectoryError::SourceDirectoryIsNotADirectory,
            DirectorySizeScanError::EntryNoLongerExists { path } => 
                DirectoryError::UnableToAccessSource {
                    error: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Scaned entry has been removed before it could be processed: {}",
                            path.display()
                        ),
                    ),
                },
            DirectorySizeScanError::UnableToAccessFile { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectorySizeScanError::UnableToAccessDirectory { error } => 
                DirectoryError::UnableToAccessSource { error },
            DirectorySizeScanError::OtherIoError { error } => 
                DirectoryError::OtherIoError { error },
        })?;

    Ok(DirectoryContentDetails {
        total_bytes: total_size_in_bytes,
        total_files: scan.files.len(),
        total_directories: scan.directories.len(),
    })
}

/// Move a directory from `source_directory_path` to `target_directory_path`.
///
/// - `source_directory_path` must point to an existing directory path.
/// - `target_directory_path` represents a path to the directory that will contain `source_directory_path`'s contents.
///
///
/// ### Target directory
/// Depending on the [`options.target_directory_rule`][DirectoryMoveOptions::target_directory_rule] option,
/// the `target_directory_path` must:
/// - [`DisallowExisting`][TargetDirectoryRule::DisallowExisting]: not exist,
/// - [`AllowEmpty`][TargetDirectoryRule::AllowEmpty]: either not exist or be empty, or,
/// - [`AllowNonEmpty`][TargetDirectoryRule::AllowNonEmpty]: either not exist, be empty, or be non-empty. Additionally,
///   the specified overwriting rules are respected (see fields).
///
///
/// ### Return value
/// Upon success, the function returns the number of files and directories that were moved
/// as well as the total amount of bytes moved, see [`FinishedDirectoryMove`].
///
/// ### Warnings
/// *Warning:* this function **does not follow symbolic links**.
pub fn move_directory<S, T>(
    source_directory_path: S,
    target_directory_path: T,
    options: DirectoryMoveOptions,
) -> Result<FinishedDirectoryMove, DirectoryError>
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

    let source_details = collect_source_directory_details(&source_directory_path)?;

    // We can attempt to simply rename the directory. This is much faster,
    // but will fail if the source and target paths aren't on the same mount point or filesystem or,
    // if on Windows, the target directory already exists.
    if validated_target_path
        .target_directory_is_empty
        .unwrap_or(true)
    {
        #[cfg(unix)]
        {
            // If the target directory exists, but is empty, we can (on Unix only)
            // directly rename the source directory to the target (this might still fail due to different mount points).
            if std::fs::rename(&source_directory_path, &target_directory_path).is_ok() {
                return Ok(FinishedDirectoryMove {
                    total_bytes_moved: source_details.total_bytes,
                    num_files_moved: source_details.total_files,
                    num_directories_moved: source_details.total_directories,
                });
            }
        }

        #[cfg(windows)]
        {
            // On Windows, `rename`'s target directory must not exist.
            if !validated_target_path.target_directory_exists
                && std::fs::rename(&source_directory_path, &target_directory_path).is_ok()
            {
                return Ok(FinishedDirectoryMove {
                    total_bytes_moved: source_details.total_bytes,
                    num_files_moved: source_details.total_files,
                    num_directories_moved: source_details.total_directories,
                });
            }

            // Otherwise, we can rename the *contents* of the directory instead.
            // Note that this is not a recursive scan, we're simply moving (by renaming)
            // the files and directories directly inside the source directory into the target directory.
            let source_directory_contents = std::fs::read_dir(&source_directory_path)
                .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

            for source_entry in source_directory_contents {
                let source_entry =
                    source_entry.map_err(|error| DirectoryError::UnableToAccessSource { error })?;

                let source_path = source_entry.path();
                let target_path = rejoin_source_subpath_onto_target(
                    &source_directory_path,
                    &source_path,
                    &validated_target_path.target_directory_path,
                )?;

                std::fs::rename(source_path, target_path)
                    .map_err(|error| DirectoryError::OtherIoError { error })?;
            }

            // Finally, we need to remove the, now empty, source directory path.
            std::fs::remove_dir(&source_directory_path)
                .map_err(|error| DirectoryError::UnableToAccessSource { error })?;

            return Ok(FinishedDirectoryMove {
                total_bytes_moved: source_details.total_bytes,
                num_files_moved: source_details.total_files,
                num_directories_moved: source_details.total_directories,
            });
        }

        #[cfg(not(any(unix, windows)))]
        {
            compile_error!(
                "fs-more supports only the following values of target_family: unix and windows \
                (notably, wasm is unsupported)."
            );
        }
    }

    // At this point a simple rename was either impossible or failed.
    // We need to copy and delete instead.

    if !validated_target_path.target_directory_exists {
        std::fs::create_dir_all(&source_directory_path)
            .map_err(|error| DirectoryError::UnableToAccessTarget { error })?;
    }

    copy_directory_unchecked(
        source_directory_path.clone(),
        validated_target_path,
        DirectoryCopyOptions {
            target_directory_rule: options.target_directory_rule,
            maximum_copy_depth: None,
        },
    )?;

    std::fs::remove_dir_all(source_directory_path)
        .map_err(|error| DirectoryError::OtherIoError { error })?;

    // TODO Test this entire method.
    Ok(FinishedDirectoryMove {
        total_bytes_moved: source_details.total_bytes,
        num_files_moved: source_details.total_files,
        num_directories_moved: source_details.total_directories,
    })
}
