use std::path::PathBuf;

use crate::{
    error::{DirectoryScanError, DirectorySizeScanError, FileSizeError},
    file::get_file_size_in_bytes,
};


/// A way to scan a directory's contents (either a single directory,
/// up to a certain subdirectory limit or the entire subtree).
///
/// **Be careful!** If you set `follow_symbolic_links` to `true` in [`Self::scan_with_options`],
/// the resulting `files` and `directories` *might not all be sub-paths of the root* `directory_path`
/// (as we follow any symbolic links leading elsewhere and include their full target path in the results).
pub struct DirectoryScan {
    /// The directory that was scanned.
    pub root_directory_path: PathBuf,

    /// The maximum depth setting used in this scan.
    ///
    /// - `None` indicates no depth limit.
    /// - `Some(0)` means a scan that returns only the files and directories directly
    /// in the root directory and doesn't scan any subdirectories.
    /// - `Some(1)` includes the root directory's contents and one level of its subdirectories.
    pub maximum_scanned_depth: Option<usize>,

    /// Indicates whether the scan that was performed wasn't deep enough to cover
    /// *all* the subdirectories (i.e. there are subdirectories deeper than the depth limit allowed).
    ///
    /// In many situations this flag being `true` isn't a bad sign. If you're intentionaly
    /// limiting the scan depth to avoid problematic scan times, this is fine.
    ///
    /// *Be warned, however*, that if you call something like [`self.total_size_in_bytes()`][Self::total_size_in_bytes],
    /// you need to keep in mind that, if this flag is `true`, the value returned by that method likely doesn't cover
    /// the *entire* contents of the directory.
    pub is_deeper_than_scan_allows: bool,

    /// Files that were found in the scan.
    pub files: Vec<PathBuf>,

    /// Directories that were found in the scan. Doesn't include the root directory.
    pub directories: Vec<PathBuf>,
}

impl DirectoryScan {
    /// Perform a new directory scan.
    ///
    /// `directory_path` must point to a directory that exists,
    /// otherwise an `Err` with [`DirectoryScanError::NotFound`][crate::error::DirectoryScanError::NotFound] is returned.
    pub fn scan_with_options<P>(
        directory_path: P,
        maximum_scan_depth: Option<usize>,
        follow_symbolic_links: bool,
    ) -> Result<Self, DirectoryScanError>
    where
        P: Into<PathBuf>,
    {
        let directory_path = directory_path.into();

        // Ensure the directory exists. We use `try_exists`
        // instead of `exists` to catch permission and other IO errors
        // as distinct from the `DirectoryScanError::NotFound` error.
        match directory_path.try_exists() {
            Ok(exists) => {
                if !exists {
                    return Err(DirectoryScanError::NotFound);
                }
            }
            Err(error) => {
                return Err(DirectoryScanError::UnableToReadDirectory { error });
            }
        }

        if !directory_path.is_dir() {
            return Err(DirectoryScanError::NotADirectory);
        }


        let mut file_list = Vec::new();
        let mut directory_list = Vec::new();
        let mut is_deeper_than_scan_allows = false;

        // Create a FIFO (queue) of directories that need to be scanned.
        let mut directory_scan_queue = Vec::new();

        struct PendingDirectoryScan {
            path: PathBuf,
            depth: usize,
        }

        impl PendingDirectoryScan {
            #[inline]
            pub fn new(path: PathBuf, depth: usize) -> Self {
                Self { path, depth }
            }
        }

        directory_scan_queue.push(PendingDirectoryScan::new(
            directory_path.clone(),
            0,
        ));

        while !directory_scan_queue.is_empty() {
            let next_directory = directory_scan_queue.pop().expect(
                "BUG: Can't pop item from Vec even though is_empty == false.",
            );

            let directory_iterator = std::fs::read_dir(&next_directory.path)
                .map_err(
                    |error| DirectoryScanError::UnableToReadDirectory { error },
                )?;

            for item in directory_iterator {
                let item = item.map_err(|error| {
                    DirectoryScanError::UnableToReadDirectoryItem { error }
                })?;

                let item_file_type = item.file_type().map_err(|error| {
                    DirectoryScanError::UnableToReadDirectoryItem { error }
                })?;

                if item_file_type.is_file() {
                    // Files are simply added to the resulting scan and no further action is needed.
                    file_list.push(item.path());
                } else if item_file_type.is_dir() {
                    // Directories might in addition to being stored in the results need
                    // to be scanned themselves, but only if the depth limit permits it.
                    // We can do that by adding them to the scan queue.
                    if let Some(maximum_depth) = maximum_scan_depth {
                        if next_directory.depth < maximum_depth {
                            directory_scan_queue.push(
                                PendingDirectoryScan::new(
                                    item.path(),
                                    next_directory.depth + 1,
                                ),
                            );
                        } else {
                            is_deeper_than_scan_allows = true;
                        }
                    } else {
                        directory_scan_queue.push(PendingDirectoryScan::new(
                            item.path(),
                            next_directory.depth + 1,
                        ));
                    }


                    directory_list.push(item.path());
                } else if item_file_type.is_symlink() && follow_symbolic_links {
                    // If an item is a symbolic link, we ignore it, unless `follow_symbolic_links` is enabled.
                    // If enabled, we follow it to its destination and append that *destination* path
                    // to the file or directory list.
                    let real_path =
                        std::fs::read_link(item.path()).map_err(|error| {
                            DirectoryScanError::UnableToReadDirectoryItem {
                                error,
                            }
                        })?;

                    if !real_path.exists() {
                        continue;
                    }

                    if real_path.is_file() {
                        file_list.push(real_path);
                    } else if real_path.is_dir() {
                        // Depth settings are respected if the destination is a directory.
                        if let Some(maximum_depth) = maximum_scan_depth {
                            if next_directory.depth < maximum_depth {
                                directory_scan_queue.push(
                                    PendingDirectoryScan::new(
                                        real_path.clone(),
                                        next_directory.depth + 1,
                                    ),
                                );
                            } else {
                                is_deeper_than_scan_allows = true;
                            }
                        } else {
                            directory_scan_queue.push(
                                PendingDirectoryScan::new(
                                    real_path.clone(),
                                    next_directory.depth + 1,
                                ),
                            );
                        }

                        directory_list.push(real_path);
                    }
                }
            }
        }

        Ok(Self {
            root_directory_path: directory_path,
            maximum_scanned_depth: maximum_scan_depth,
            is_deeper_than_scan_allows,
            files: file_list,
            directories: directory_list,
        })
    }

    /// Returns a total size of the scanned files in bytes.
    ///
    /// *Be careful:* This goes over all the scanned files and queries their size.
    /// This means you get an up-to-date directory size if you call this multiple times
    /// after modifying the files, but it also means that it will return an `Err` with
    /// [`DirectorySizeScanError::FileNoLongerExists`][crate::error::DirectorySizeScanError::FileNoLongerExists]
    /// if some file that was previously scanned has been removed since.
    ///
    /// *Be careful:* if you initialized [`DirectoryScan`][Self] with a depth parameter
    /// that is smaller than the actual depth of the directory tree you're scanning,
    /// the value returned by this function will be smaller than
    /// the entire contents of the directory. For more information, see the
    /// [`is_deeper_than_scan_allows`][Self::is_deeper_than_scan_allows] field.
    pub fn total_size_in_bytes(&self) -> Result<u64, DirectorySizeScanError> {
        let mut total_bytes = 0;

        for file_path in &self.files {
            let file_size_bytes = get_file_size_in_bytes(file_path).map_err(
                |error| match error {
                    FileSizeError::NotFound => {
                        DirectorySizeScanError::FileNoLongerExists
                    }
                    FileSizeError::NotAFile => {
                        DirectorySizeScanError::FileNoLongerExists
                    }
                    FileSizeError::UnableToAccessFile { error } => {
                        DirectorySizeScanError::UnableToAccessFile { error }
                    }
                    FileSizeError::OtherIoError { error } => {
                        DirectorySizeScanError::OtherIoError { error }
                    }
                },
            )?;

            total_bytes += file_size_bytes;
        }

        Ok(total_bytes)
    }
}
