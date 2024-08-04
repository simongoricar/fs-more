use std::path::Path;

use super::{DirectoryScanDepthLimit, DirectoryScanOptions, DirectoryScanner};
use crate::error::DirectoryScanError;

pub(crate) struct DirectoryStatistics {
    pub(crate) total_bytes: u64,
    pub(crate) total_files: usize,
    pub(crate) total_directories: usize,
}

pub(crate) fn collect_directory_statistics_via_scan(
    directory_path: &Path,
) -> Result<DirectoryStatistics, DirectoryScanError> {
    let scanner = DirectoryScanner::new(
        directory_path,
        DirectoryScanOptions {
            yield_base_directory: false,
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: false,
            follow_base_directory_symbolic_link: true,
        },
    )
    .into_iter();


    let mut total_bytes = 0;
    let mut total_files = 0;
    let mut total_directories = 0;

    for scan_entry_result in scanner {
        let scan_entry = scan_entry_result?;

        let scan_entry_size_bytes = scan_entry.metadata().len();
        let scan_entry_file_type = scan_entry.metadata().file_type();

        total_bytes += scan_entry_size_bytes;

        if scan_entry_file_type.is_file() {
            total_files += 1;
        } else if scan_entry_file_type.is_dir() {
            total_directories += 1;
        }
    }


    Ok(DirectoryStatistics {
        total_bytes,
        total_files,
        total_directories,
    })
}
