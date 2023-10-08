#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "fs-err")]
use fs_err as fs;

use crate::{
    error::{DirectoryIsEmptyError, DirectoryScanError, DirectorySizeScanError, FileSizeError},
    file::file_size_in_bytes,
};


/// A directory scanner abstraction.
///
/// ### Scan depth
/// Maximum scanning depth can be configured by setting
/// the `maximum_scan_depth` parameter in the [`DirectoryScan::scan_with_options`] initializer to:
/// - `Some(0)` -- scans direct contents of the directory (a single layer of files and directories),
/// - `Some(1+)` -- scans up to a certain subdirectory limit, or,
/// - `None` -- scans the entire subtree, as deep as required.
///
/// ### Symbolic links
/// **Careful!** This scanner follows symbolic links.
///
/// This means that if you set the `follow_symbolic_links` option to `true` (see [`Self::scan_with_options`]),
/// the resulting `files` and `directories` included in the scan results
/// *might not all be sub-paths of the root* `directory_path`.
///
/// This is because we followed symbolic links included their full target path in the results,
/// not their original path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryScan {
    /// The directory that was scanned.
    pub(crate) root_directory_path: PathBuf,

    /// The maximum depth setting used in this scan.
    ///
    /// - `None` indicates no depth limit.
    /// - `Some(0)` means a scan that returns only the files and directories directly
    /// in the root directory and doesn't scan any subdirectories.
    /// - `Some(1)` includes the root directory's contents and one level of its subdirectories.
    pub(crate) maximum_scanned_depth: Option<usize>,

    /// Indicates whether the scan that was performed wasn't deep enough to cover
    /// all of the files and subdirectories (i.e. there are subdirectories deeper than the depth limit allowed).
    ///
    /// In some situations this flag being `true` isn't a bad sign -- if you're intentionaly
    /// limiting the scan depth to avoid problematic scan times or other reasons, this is likely fine.
    ///
    /// Also note that if you set `maximum_scan_depth` to `None`, *this flag can never be `true`*.
    ///
    /// ## Warning
    /// *Be warned, however*, that if you call e.g. the [`self.total_size_in_bytes()`][Self::total_size_in_bytes] method,
    /// you need to keep in mind that, if this flag is `true`, the number of bytes likely doesn't cover
    /// the *entire* contents of the directory (as they weren't scanned). To get the correct size of a directory
    /// and its contents, ideally perform a scan without a depth limit and then use the [`self.total_size_in_bytes()`][Self::total_size_in_bytes]
    /// method as before to get the correct result.
    pub is_real_directory_deeper_than_scan: bool,

    /// Files that were found in the scan.
    pub files: Vec<PathBuf>,

    /// Directories that were found in the scan. Doesn't include the root directory.
    pub directories: Vec<PathBuf>,
}

impl DirectoryScan {
    /// Perform a directory scan.
    ///
    /// `directory_path` must point to a directory that exists,
    /// otherwise an `Err(`[`DirectoryScanError::NotFound`][crate::error::DirectoryScanError::NotFound]`)` is returned.
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

        while let Some(next_directory) = directory_scan_queue.pop() {
            let directory_iterator = fs::read_dir(&next_directory.path)
                .map_err(|error| DirectoryScanError::UnableToReadDirectory { error })?;

            for item in directory_iterator {
                let item =
                    item.map_err(|error| DirectoryScanError::UnableToReadDirectoryItem { error })?;

                let item_file_type = item
                    .file_type()
                    .map_err(|error| DirectoryScanError::UnableToReadDirectoryItem { error })?;

                if item_file_type.is_file() {
                    // Files are simply added to the resulting scan and no further action is needed.
                    file_list.push(item.path());
                } else if item_file_type.is_dir() {
                    // Directories might in addition to being stored in the results need
                    // to be scanned themselves, but only if the depth limit permits it.
                    // We can do that by adding them to the scan queue.
                    if let Some(maximum_depth) = maximum_scan_depth {
                        if next_directory.depth < maximum_depth {
                            directory_scan_queue.push(PendingDirectoryScan::new(
                                item.path(),
                                next_directory.depth + 1,
                            ));
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
                    let real_path = fs::read_link(item.path())
                        .map_err(|error| DirectoryScanError::UnableToReadDirectoryItem { error })?;

                    if !real_path.exists() {
                        continue;
                    }

                    if real_path.is_file() {
                        file_list.push(real_path);
                    } else if real_path.is_dir() {
                        // Depth settings are respected if the destination is a directory.
                        if let Some(maximum_depth) = maximum_scan_depth {
                            if next_directory.depth < maximum_depth {
                                directory_scan_queue.push(PendingDirectoryScan::new(
                                    real_path.clone(),
                                    next_directory.depth + 1,
                                ));
                            } else {
                                is_deeper_than_scan_allows = true;
                            }
                        } else {
                            directory_scan_queue.push(PendingDirectoryScan::new(
                                real_path.clone(),
                                next_directory.depth + 1,
                            ));
                        }

                        directory_list.push(real_path);
                    }
                }
            }
        }

        Ok(Self {
            root_directory_path: directory_path,
            maximum_scanned_depth: maximum_scan_depth,
            is_real_directory_deeper_than_scan: is_deeper_than_scan_allows,
            files: file_list,
            directories: directory_list,
        })
    }


    /// Returns a slice of all scanned files (items are full file paths).
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Returns a slice of all scanned directories (items are full directory paths).
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Returns a total size of the scanned files in bytes.
    ///
    /// *Be careful:* This goes over all the scanned files and directories and queries their size.
    /// This means you get a fully up-to-date directory size if you happen to call this multiple times
    /// after modifying the files, but it also means that it will return an `Err` with
    /// [`DirectorySizeScanError::EntryNoLongerExists`][crate::error::DirectorySizeScanError::EntryNoLongerExists]
    /// if some file or directory that was previously scanned has been removed since.
    ///
    /// *Be careful:* if you initialized [`DirectoryScan`][Self] with a depth parameter
    /// that is smaller than the actual depth of the directory tree you're scanning,
    /// the value returned by this function will be smaller than
    /// the entire contents of the directory. For more information, see the
    /// [`is_deeper_than_scan_allows`][Self::is_deeper_than_scan_allows] field.
    pub fn total_size_in_bytes(&self) -> Result<u64, DirectorySizeScanError> {
        let mut total_bytes = 0;

        for file_path in &self.files {
            let file_size_bytes = file_size_in_bytes(file_path).map_err(|error| match error {
                FileSizeError::NotFound => DirectorySizeScanError::EntryNoLongerExists {
                    path: file_path.clone(),
                },
                FileSizeError::NotAFile => DirectorySizeScanError::EntryNoLongerExists {
                    path: file_path.clone(),
                },
                FileSizeError::UnableToAccessFile { error } => {
                    DirectorySizeScanError::UnableToAccessFile { error }
                }
                FileSizeError::OtherIoError { error } => {
                    DirectorySizeScanError::OtherIoError { error }
                }
            })?;

            total_bytes += file_size_bytes;
        }

        for directory_path in &self.directories {
            let directory_size_bytes = fs::metadata(directory_path)
                .map_err(|_| DirectorySizeScanError::EntryNoLongerExists {
                    path: directory_path.to_path_buf(),
                })?
                .len();

            total_bytes += directory_size_bytes;
        }

        Ok(total_bytes)
    }
}

/// Returns `Ok(true)` if the given directory is completely empty, `Ok(false)` otherwise.
///
/// Does not check whether the path exists, meaning the error return type is
/// a very uninformative [`std::io::Error`].
pub(crate) fn is_directory_empty_unchecked(directory_path: &Path) -> std::io::Result<bool> {
    let mut directory_read = fs::read_dir(directory_path)?;
    Ok(directory_read.next().is_none())
}

/// Returns a `bool` indicating whether the given directory is completely empty (no files and no subdirectories).
pub fn is_directory_empty<P>(directory_path: P) -> Result<bool, DirectoryIsEmptyError>
where
    P: AsRef<Path>,
{
    let directory_path: &Path = directory_path.as_ref();
    let directory_metadata =
        fs::metadata(directory_path).map_err(|_| DirectoryIsEmptyError::NotFound)?;

    if !directory_metadata.is_dir() {
        return Err(DirectoryIsEmptyError::NotADirectory);
    }


    let mut directory_read = fs::read_dir(directory_path)
        .map_err(|error| DirectoryIsEmptyError::UnableToReadDirectory { error })?;

    Ok(directory_read.next().is_some())
}
