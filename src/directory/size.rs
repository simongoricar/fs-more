use std::path::PathBuf;

use super::{DirectoryScanDepthLimit, DirectoryScanOptions};
use crate::{
    directory::DirectoryScan,
    error::{DirectoryScanError, DirectorySizeScanError},
};


/// Returns the size of the directory, including all of its files and subdirectories, in bytes.
///
/// There is no limit to the depth of this scan; the directory tree is traversed as deep as needed.
///
///
/// This function is essentially a shortcut for initializing
/// a [`DirectoryScan`] with unlimited scan depth and calling its
/// [`total_size_in_bytes`][DirectoryScan::total_size_in_bytes] method
/// immediately after.
pub fn directory_size_in_bytes<P>(
    directory_path: P,
    follow_symbolic_links: bool,
) -> Result<u64, DirectorySizeScanError>
where
    P: Into<PathBuf>,
{
    let unlimited_depth_scan = DirectoryScan::scan_with_options(
        directory_path,
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links,
        },
    )
    .map_err(|error| match error {
        DirectoryScanError::NotFound { path } => {
            DirectorySizeScanError::ScanDirectoryNotFound { path }
        }
        DirectoryScanError::NotADirectory { path } => {
            DirectorySizeScanError::ScanDirectoryNotADirectory { path }
        }
        DirectoryScanError::UnableToReadDirectory {
            directory_path,
            error,
        } => DirectorySizeScanError::UnableToAccessDirectory {
            directory_path,
            error,
        },
        DirectoryScanError::UnableToReadDirectoryItem {
            directory_path,
            error,
        } => DirectorySizeScanError::UnableToAccessFile {
            file_path: directory_path,
            error,
        },
    })?;

    unlimited_depth_scan.total_size_in_bytes()
}
