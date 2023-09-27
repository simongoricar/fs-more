use std::path::PathBuf;

use crate::{
    directory::DirectoryScan,
    error::{DirectoryScanError, DirectorySizeScanError},
};


/// Returns the size of the directory and all its files
/// (including files in its subdirectories).
///
/// There is no depth limit, the directory tree is traversed as far as needed.
///
/// *Note:* this is essentially nothing more than a shortcut for initializing
/// a [`DirectoryScan`] with unlimited depth and calling the
/// [`total_size_in_bytes`][DirectoryScan::total_size_in_bytes] method.
pub fn get_directory_size<P>(
    directory_path: P,
    follow_symbolic_links: bool,
) -> Result<u64, DirectorySizeScanError>
where
    P: Into<PathBuf>,
{
    let unlimited_depth_scan =
        DirectoryScan::scan_with_options(directory_path, None, follow_symbolic_links).map_err(
            |error| match error {
                DirectoryScanError::NotFound => DirectorySizeScanError::RootDirectoryNotFound,
                DirectoryScanError::NotADirectory => DirectorySizeScanError::RootIsNotADirectory,
                DirectoryScanError::UnableToReadDirectory { error } => {
                    DirectorySizeScanError::UnableToAccessDirectory { error }
                }
                DirectoryScanError::UnableToReadDirectoryItem { error } => {
                    DirectorySizeScanError::UnableToAccessFile { error }
                }
            },
        )?;

    unlimited_depth_scan.total_size_in_bytes()
}
