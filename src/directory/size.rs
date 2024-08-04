use std::path::PathBuf;

use super::{DirectoryScanDepthLimit, DirectoryScanOptionsV2, DirectoryScanner};
use crate::error::DirectorySizeScanErrorV2;


/// Returns the size of the directory, including all of its files and subdirectories, in bytes.
///
/// There is no limit to the depth of this scan; the directory tree is traversed as deep as needed.
///
///
/// This function is essentially a shortcut for initializing
/// a [`DirectoryScanner`] with unlimited scan depth and summing entries' sizes.
pub fn directory_size_in_bytes<P>(directory_path: P) -> Result<u64, DirectorySizeScanErrorV2>
where
    P: Into<PathBuf>,
{
    let directory_path: PathBuf = directory_path.into();


    let unlimited_depth_scan = DirectoryScanner::new(
        &directory_path,
        DirectoryScanOptionsV2 {
            yield_base_directory: true,
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: false,
            follow_base_directory_symbolic_link: true,
        },
    );


    let mut total_bytes = 0;

    for scan_entry_result in unlimited_depth_scan.into_iter() {
        let scan_entry =
            scan_entry_result.map_err(|error| DirectorySizeScanErrorV2::ScanError {
                error,
                directory_path: directory_path.clone(),
            })?;

        let entry_size_in_bytes = scan_entry.into_metadata().len();

        total_bytes += entry_size_in_bytes;
    }

    Ok(total_bytes)
}
