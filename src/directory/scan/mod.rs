use std::{
    fs::Metadata,
    path::{Path, PathBuf},
};


use_enabled_fs_module!();

use crate::error::{DirectoryEmptinessScanErrorV2, DirectoryScanErrorV2};

pub(crate) mod collected;
mod iter;
pub use iter::*;


/*
/// A list of file and directory paths.
///
/// You can obtain this from [`DirectoryScan::into_scanned_files_and_directories`].
#[deprecated]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScannedFilesAndDirectories {
    /// Scanned files (their paths).
    pub files: Vec<PathBuf>,

    /// Scanned directories (their paths).
    pub directories: Vec<PathBuf>,
}
 */


/// The maximum directory scan depth option.
///
/// Used primarily in [`DirectoryScan::scan_with_options`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DirectoryScanDepthLimit {
    /// No scan depth limit.
    Unlimited,

    /// Scan depth is limited to `maximum_depth`, where the value refers to
    /// the maximum depth of the subdirectory whose contents are still listed.
    ///
    ///
    /// # Examples
    /// `maximum_depth = 0` indicates a scan that will cover only the files and directories
    /// directly in the source directory.
    ///
    /// ```md
    /// ~/scanned-directory
    ///  |- foo.csv
    ///  |- foo-2.csv
    ///  |- bar/
    ///     (no entries listed)
    /// ```
    ///
    /// Notice how *contents* of the `~/scanned-directory/bar/`
    /// directory are not returned in the scan when using depth `0`.
    ///
    ///
    /// <br>
    ///
    /// `maximum_depth = 1` will cover the files and directories directly in the source directory
    /// plus one level of files and subdirectories deeper.
    ///
    /// ```md
    /// ~/scanned-directory
    ///  |- foo.csv
    ///  |- foo-2.csv
    ///  |- bar/
    ///     |- hello-world.txt
    ///     |- bar2/
    ///        (no entries listed)
    /// ```
    ///
    /// Notice how contents of `~/scanned-directory/bar` are listed,
    /// but contents of `~/scanned-directory/bar/bar2` are not.
    Limited {
        /// Maximum scan depth.
        maximum_depth: usize,
    },
}

/*

/// Options that influence [`DirectoryScan`].
#[derive(Clone, PartialEq, Eq, Debug)]
#[deprecated]
pub struct DirectoryScanOptions {
    /// The maximum directory scanning depth, see [`DirectoryScanDepthLimit`].
    pub maximum_scan_depth: DirectoryScanDepthLimit,

    /// Whether to follow symbolic links when scanning or not.
    ///
    /// ## If enabled
    /// We'll follow the symbolic links, even if they lead outside the base `directory_path`.
    /// Note that this means the files and directories included in the scan results
    /// **might not necessarily be sub-paths of the provided base `directory_path`**.
    ///
    /// If a symbolic link turns out to be broken (its destination doesn't exist),
    /// it is simply ignored (not included in the scan results).
    ///
    ///
    /// ## If disabled
    /// When we encounter a symbolic link, the results will include the file path of
    /// the symbolic link itself, *not the link's destination path*.
    ///
    /// If an encountered symbolic link points to a directory, it will
    /// be included in the results in a similar manner, but with one significant difference:
    /// as we won't resolve symbolic links, the files and subdirectories of that symlinked directory
    /// will not be scanned, even if the scan depth limit would have allowed it.
    ///
    /// If a symbolic link turns out to be broken (its destination doesn't exist),
    /// it is simply ignored (not included in the scan results).
    pub follow_symbolic_links: bool,
}

impl Default for DirectoryScanOptions {
    /// Returns the default directory scanning options, which are:
    /// - unlimited scan depth,
    /// - symlinks are no followed.
    fn default() -> Self {
        Self {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: false,
        }
    }
}
 */



// #[derive(Clone, PartialEq, Eq, Debug)]
// pub enum DirectoryScanTraversalMode {
//     BreadthFirst,
//     DepthFirst,
// }



/// Options that influence [`DirectoryScan`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryScanOptionsV2 {
    /// Whether to have the iterator yield the base directory
    /// as its first item or not.
    pub yield_base_directory: bool,

    /// The maximum directory scanning depth, see [`DirectoryScanDepthLimit`].
    /// TODO Not implemented yet.
    pub maximum_scan_depth: DirectoryScanDepthLimit,

    // pub iteration_order: DirectoryScanIterationOrder,
    // pub traversal_mode: DirectoryScanTraversalMode,
    /// TODO Not implemented yet.
    pub follow_symbolic_links: bool,

    /// If enabled, and if the base directory is a symbolic link,
    /// the iterator will first resolve the symbolic link,
    /// then proceed with scanning the destination. If the symbolic link
    /// does not point to a directory, an error will be returned from
    /// the first call to iterator's [`next`].
    ///
    /// If disabled, and the base directory is a symbolic link,
    /// the iterator will either yield only the base directory
    /// (if `yield_base_directory` is true), or nothing.
    ///
    /// This has no effect if the base directory is not a symlink.
    ///
    ///
    /// [`next`]: DirectoryScannerPerDirectoryIter::next
    pub follow_base_directory_symbolic_link: bool,
}

impl Default for DirectoryScanOptionsV2 {
    fn default() -> Self {
        Self {
            yield_base_directory: true,
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: false,
            follow_base_directory_symbolic_link: false,
        }
    }
}



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanEntryDepth {
    BaseDirectory,
    AtDepth { depth: usize },
}

impl ScanEntryDepth {
    fn plus_one_level(self) -> Self {
        match self {
            ScanEntryDepth::BaseDirectory => ScanEntryDepth::AtDepth { depth: 0 },
            ScanEntryDepth::AtDepth { depth } => ScanEntryDepth::AtDepth { depth: depth + 1 },
        }
    }
}



pub struct ScanEntry {
    path: PathBuf,

    metadata: Metadata,

    depth: ScanEntryDepth,
}

impl ScanEntry {
    #[inline]
    fn new(path: PathBuf, metadata: Metadata, depth: ScanEntryDepth) -> Self {
        Self {
            path,
            metadata,
            depth,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }

    pub fn into_metadata(self) -> Metadata {
        self.metadata
    }
}




/// A directory scanner with configurable iteration behaviour.
///
///
/// # Alternatives
///
/// This scanner is able to recursively iterate over the directory
/// as well as optionally follow symbolic links. If, however, you're
/// looking for something with a bit more features, such as sorting,
/// and a longer history of ecosystem use, consider the
/// [`walkdir`](https://docs.rs/walkdir) crate.
pub struct DirectoryScanner {
    base_path: PathBuf,

    options: DirectoryScanOptionsV2,
}

impl DirectoryScanner {
    pub fn new<P>(base_directory_path: P, options: DirectoryScanOptionsV2) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            base_path: base_directory_path.into(),
            options,
        }
    }
}

impl IntoIterator for DirectoryScanner {
    type IntoIter = DirectoryScannerPerDirectoryIter;
    type Item = Result<ScanEntry, DirectoryScanErrorV2>;

    fn into_iter(self) -> Self::IntoIter {
        DirectoryScannerPerDirectoryIter::new(self.base_path, self.options)
    }
}


/*


/// A directory scanner with configurable scan depth and symlink behaviour.
///
/// This scanner is able to recursively iterate over the directory
/// as well as optionally follow symbolic links. If, however, you're
/// looking for something with a bit more features, such as lazy iteration
/// and sorting, consider the [`walkdir`](https://docs.rs/walkdir) crate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[deprecated]
pub struct DirectoryScan {
    /// Path of the directory that was scanned.
    root_directory_path: PathBuf,

    /// A `bool` indicating whether this scan covered the entire directory tree.
    ///
    /// For example, this can be `false` when the user limits the scan depth
    /// to e.g. [`DirectoryScanDepth::Limited`]`{ maximum_depth: 1 }`,
    /// but the actual directory structure has e.g. three layers of subdirectories and files.
    ///
    /// If `maximum_scan_depth` is set to
    /// [`DirectoryScanDepth::Unlimited`][]
    /// in the constructor for this scan, this method will always return `true`.
    covers_entire_subtree: bool,

    /// Files that were found in the scan.
    files: Vec<PathBuf>,

    /// Directories that were found in the scan.
    /// Doesn't include the root directory (`root_directory_path`).
    directories: Vec<PathBuf>,
}

impl DirectoryScan {
    /// Perform a directory scan.
    ///
    ///
    /// # Scan depth
    /// Maximum scanning depth can be configured by setting
    /// [`options.maximum_scan_depth`].
    ///
    ///
    /// # Symbolic links
    /// This scanner can follow symbolic links, see [`options.follow_symbolic_links`]
    /// for more information.
    ///
    ///
    /// ## `directory_path` symlink behaviour
    /// Regardless of the symbolic link option described above:
    /// if `directory_path` itself is a symbolic link to a directory,
    /// the link destination will be resolved before beginning the scan.
    ///
    ///
    /// [`options.follow_symbolic_links`]: DirectoryScanOptions::follow_symbolic_links
    /// [`options.maximum_scan_depth`]: DirectoryScanOptions::maximum_scan_depth
    pub fn scan_with_options<P>(
        directory_path: P,
        options: DirectoryScanOptions,
    ) -> Result<Self, DirectoryScanError>
    where
        P: Into<PathBuf>,
    {
        let directory_path: PathBuf = directory_path.into();


        // Ensure the directory exists. We use `try_exists`
        // instead of `exists` to catch permission and other IO errors
        // as distinct from the `DirectoryScanError::NotFound` error.

        match directory_path.try_exists() {
            Ok(exists) => {
                if !exists {
                    return Err(DirectoryScanError::NotFound {
                        path: directory_path,
                    });
                }
            }
            Err(error) => {
                return Err(DirectoryScanError::UnableToReadDirectory {
                    directory_path,
                    error,
                });
            }
        }

        if !directory_path.is_dir() {
            return Err(DirectoryScanError::NotADirectory {
                path: directory_path,
            });
        }


        let mut file_list = Vec::new();
        let mut directory_list = Vec::new();
        let mut actual_tree_is_deeper_than_scan = false;


        // Create a FIFO (queue) of directories that need to be scanned.

        struct PendingDirectoryScan {
            /// The directory to scan.
            path: PathBuf,

            /// How deep the directory is. The initial `directory_path` has a depth of `0`,
            /// a direct directory descendant in it has `1`, and so on.
            depth: usize,
        }

        impl PendingDirectoryScan {
            #[inline]
            pub fn new(path: PathBuf, depth: usize) -> Self {
                Self { path, depth }
            }
        }


        let mut directory_scan_queue = Vec::new();

        directory_scan_queue.push(PendingDirectoryScan::new(directory_path.clone(), 0));


        while let Some(next_directory) = directory_scan_queue.pop() {
            let directory_entry_iterator = fs::read_dir(&next_directory.path).map_err(|error| {
                DirectoryScanError::UnableToReadDirectory {
                    directory_path: next_directory.path.clone(),
                    error,
                }
            })?;


            for directory_entry in directory_entry_iterator {
                let directory_entry = directory_entry.map_err(|error| {
                    DirectoryScanError::UnableToReadDirectoryItem {
                        directory_path: next_directory.path.clone(),
                        error,
                    }
                })?;

                let item_file_type = directory_entry.file_type().map_err(|error| {
                    DirectoryScanError::UnableToReadDirectoryItem {
                        directory_path: next_directory.path.clone(),
                        error,
                    }
                })?;


                if item_file_type.is_file() {
                    // Files are simply added to the resulting scan and no further action is needed.

                    file_list.push(directory_entry.path());
                } else if item_file_type.is_dir() {
                    // If the scan depth limit allows it, sub-directories will need to be scanned
                    // for additional content. We can do that by adding them to the `directory_scan_queue`.

                    match options.maximum_scan_depth {
                        DirectoryScanDepthLimit::Limited { maximum_depth } => {
                            if next_directory.depth < maximum_depth {
                                directory_scan_queue.push(PendingDirectoryScan::new(
                                    directory_entry.path(),
                                    next_directory.depth + 1,
                                ));
                            } else {
                                // This marks down that we weren't able to scan the
                                // full directory tree due to scan depth limits.
                                actual_tree_is_deeper_than_scan = true;
                            }
                        }
                        DirectoryScanDepthLimit::Unlimited => {
                            directory_scan_queue.push(PendingDirectoryScan::new(
                                directory_entry.path(),
                                next_directory.depth + 1,
                            ));
                        }
                    }

                    directory_list.push(directory_entry.path());
                } else if item_file_type.is_symlink() {
                    // If `follow_symbolic_links` is set to `true`, we follow the link to its destination
                    // and append that *destination* path to the file or directory list,
                    // incrementing the depth as we would for normal directories.

                    // If it is set to `false`, we find whether it points to a file or a directory,
                    // then just include the original non-resolved path in the results.

                    let resolved_symlink_path =
                        fs::read_link(directory_entry.path()).map_err(|error| {
                            DirectoryScanError::UnableToReadDirectoryItem {
                                directory_path: next_directory.path.clone(),
                                error,
                            }
                        })?;

                    match resolved_symlink_path.try_exists() {
                        Ok(exists) => {
                            if !exists {
                                continue;
                            }
                        }
                        Err(error) => {
                            return Err(DirectoryScanError::UnableToReadDirectoryItem {
                                directory_path: next_directory.path.clone(),
                                error,
                            });
                        }
                    }

                    let resolved_symlink_metadata =
                        fs::metadata(&resolved_symlink_path).map_err(|error| {
                            DirectoryScanError::UnableToReadDirectoryItem {
                                directory_path: next_directory.path.clone(),
                                error,
                            }
                        })?;


                    if options.follow_symbolic_links {
                        if resolved_symlink_metadata.is_file() {
                            file_list.push(resolved_symlink_path);
                        } else if resolved_symlink_metadata.is_dir() {
                            // Depth settings are respected if the destination is a directory.
                            match options.maximum_scan_depth {
                                DirectoryScanDepthLimit::Limited { maximum_depth } => {
                                    if next_directory.depth < maximum_depth {
                                        directory_scan_queue.push(PendingDirectoryScan::new(
                                            resolved_symlink_path.clone(),
                                            next_directory.depth + 1,
                                        ));
                                    } else {
                                        actual_tree_is_deeper_than_scan = true;
                                    }
                                }
                                DirectoryScanDepthLimit::Unlimited => {
                                    directory_scan_queue.push(PendingDirectoryScan::new(
                                        resolved_symlink_path.clone(),
                                        next_directory.depth + 1,
                                    ));
                                }
                            }

                            directory_list.push(resolved_symlink_path);
                        }
                    } else if resolved_symlink_metadata.is_file() {
                        file_list.push(directory_entry.path());
                    } else if resolved_symlink_metadata.is_dir() {
                        directory_list.push(directory_entry.path());
                    }
                }
            }
        }

        Ok(Self {
            root_directory_path: directory_path,
            covers_entire_subtree: !actual_tree_is_deeper_than_scan,
            files: file_list,
            directories: directory_list,
        })
    }


    /// Returns a slice of all scanned files (paths are absolute).
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Consumes `self` and returns a [`Vec`] containing all scanned files (paths are absolute).
    ///
    /// If you are also interested in directories, look at [`Self::files`] + [`Self::directories`]
    /// or [`Self::into_scanned_files_and_directories`] instead.
    pub fn into_files(self) -> Vec<PathBuf> {
        self.files
    }

    /// Returns a slice of all scanned directories (paths are absolute).
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Consumes `self` and returns a [`Vec`] containing all scanned directories (paths are absolute).
    ///
    /// If you are also interested in files, look at [`Self::files`] + [`Self::directories`]
    /// or [`Self::into_scanned_files_and_directories`] instead.
    pub fn into_directories(self) -> Vec<PathBuf> {
        self.files
    }

    /// Consumes `self` and returns a small struct containing two fields: `files` and `directories`.
    ///
    /// Use this method when you wish to consume the scanner and are interested in both scanned files and directories.
    /// Alternatives that don't consume the scanner are [`Self::files`] and [`Self::directories`].
    pub fn into_scanned_files_and_directories(self) -> ScannedFilesAndDirectories {
        ScannedFilesAndDirectories {
            files: self.files,
            directories: self.directories,
        }
    }

    /// Returns the total size in bytes of all scanned files and directories.
    ///
    ///
    /// ## Potential file system race conditions
    /// *Careful:* this method iterates over the scanned files and directories and queries their size at call time.
    /// This means the caller will get an up-to-date directory size if they happen to call the method multiple times,
    /// potentially after modifying the one of the scanned files.
    ///
    /// However, it also means that it this method *can* return, among other things,
    /// an `Err(`[`DirectorySizeScanError::ScanEntryNoLongerExists`]`)`
    /// if any file or directory that was scanned at initialization has been removed since.
    /// The same applies for files changing their read permissions, with that usually resulting in
    /// `Err(`[`DirectorySizeScanError::UnableToAccessFile`]`)`.
    ///
    /// This is very much the same thing as the relatively well-known file system race condition
    /// inherent in `if file_exists(): then open_file()`
    /// ([time-of-check, time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)),
    /// just on a bigger scale.
    ///
    /// The impact of this is---in most cases---relatively low, but it is worth noting.
    ///
    ///
    /// ## Impacts of scan depth limits
    /// *Careful:* if you initialized [`DirectoryScan`] with a scan depth limit
    /// that is smaller than the actual depth of the directory tree you're scanning,
    /// the value returned by this function will be smaller than
    /// the "real" contents of that directory.
    ///
    /// It is up to the user to decide whether that is desired behavior or not.
    /// To find out whether the returned number of bytes will not reflect the full depth
    /// of the directory structure, see [`Self::covers_entire_directory_tree`].
    pub fn total_size_in_bytes(&self) -> Result<u64, DirectorySizeScanError> {
        let mut total_bytes = 0;

        for file_path in &self.files {
            let file_size_bytes = file_size_in_bytes(file_path).map_err(|error| match error {
                FileSizeError::NotFound { path } => {
                    DirectorySizeScanError::ScanEntryNoLongerExists { path }
                }
                FileSizeError::NotAFile { path } => {
                    DirectorySizeScanError::ScanEntryNoLongerExists { path }
                }
                FileSizeError::UnableToAccessFile { file_path, error } => {
                    DirectorySizeScanError::UnableToAccessFile { file_path, error }
                }
                FileSizeError::OtherIoError { error } => {
                    DirectorySizeScanError::OtherIoError { error }
                }
            })?;

            total_bytes += file_size_bytes;
        }

        for directory_path in &self.directories {
            let directory_size_bytes = fs::metadata(directory_path)
                .map_err(|_| DirectorySizeScanError::ScanEntryNoLongerExists {
                    path: directory_path.to_path_buf(),
                })?
                .len();

            total_bytes += directory_size_bytes;
        }

        Ok(total_bytes)
    }

    /// Returns a `bool` indicating whether this scan covered the entire directory tree.
    ///
    /// For example, this can be `false` when the user limits the scan depth
    /// to e.g. [`DirectoryScanDepthLimit::Limited`]`{ maximum_depth: 1 }`,
    /// but the actual directory structure has e.g. three layers of subdirectories and files.
    ///
    /// If `maximum_scan_depth` is set to
    /// [`DirectoryScanDepthLimit::Unlimited`][]
    /// in the constructor for this scan, this method will always return `true`.
    pub fn covers_entire_directory_tree(&self) -> bool {
        self.covers_entire_subtree
    }
}



/// Returns `Ok(true)` if the given directory is completely empty, `Ok(false)` is it is not,
/// `Err(_)` if the read fails.
///
/// Does not check whether the path exists, meaning the error return type is
/// a very uninformative [`std::io::Error`].
pub(crate) fn is_directory_empty_unchecked(directory_path: &Path) -> std::io::Result<bool> {
    let mut directory_read = fs::read_dir(directory_path)?;
    Ok(directory_read.next().is_none())
}


/// Returns a `bool` indicating whether the given directory is completely empty.
///
/// Permission and other errors will *not* be coerced into `false`, but will raise a distinct error,
/// see [`IsDirectoryEmptyError`].
pub fn is_directory_empty<P>(directory_path: P) -> Result<bool, IsDirectoryEmptyError>
where
    P: AsRef<Path>,
{
    let directory_path: &Path = directory_path.as_ref();
    let directory_metadata =
        fs::metadata(directory_path).map_err(|_| IsDirectoryEmptyError::NotFound {
            directory_path: directory_path.to_path_buf(),
        })?;

    if !directory_metadata.is_dir() {
        return Err(IsDirectoryEmptyError::NotADirectory {
            path: directory_path.to_path_buf(),
        });
    }


    let mut directory_read = fs::read_dir(directory_path).map_err(|error| {
        IsDirectoryEmptyError::UnableToReadDirectory {
            directory_path: directory_path.to_path_buf(),
            error,
        }
    })?;

    Ok(directory_read.next().is_some())
}
 */


/// Returns `Ok(true)` if the given directory is completely empty, `Ok(false)` is it is not,
/// `Err(_)` if the read fails.
///
/// Does not check whether the path exists or whether it is actually a directory,
/// meaning the error return type is a very uninformative [`std::io::Error`].
///
/// Intended for internal use.
pub(crate) fn is_directory_empty_unchecked(directory_path: &Path) -> std::io::Result<bool> {
    let mut directory_read = fs::read_dir(directory_path)?;

    let Some(first_entry_result) = directory_read.next() else {
        return Ok(true);
    };

    first_entry_result?;

    Ok(false)
}


/// Returns a `bool` indicating whether the given directory is completely empty.
///
/// Permission and other errors will *not* be coerced into `false`, but will raise a distinct error,
/// see [`IsDirectoryEmptyError`].
///
/// TODO needs more tests!
pub fn is_directory_empty<P>(directory_path: P) -> Result<bool, DirectoryEmptinessScanErrorV2>
where
    P: AsRef<Path>,
{
    let directory_path: &Path = directory_path.as_ref();

    let directory_metadata =
        fs::metadata(directory_path).map_err(|_| DirectoryEmptinessScanErrorV2::NotFound {
            path: directory_path.to_path_buf(),
        })?;

    if !directory_metadata.is_dir() {
        return Err(DirectoryEmptinessScanErrorV2::NotADirectory {
            path: directory_path.to_path_buf(),
        });
    }


    let mut directory_read = fs::read_dir(directory_path).map_err(|error| {
        DirectoryEmptinessScanErrorV2::UnableToReadDirectory {
            directory_path: directory_path.to_path_buf(),
            error,
        }
    })?;


    let Some(first_entry_result) = directory_read.next() else {
        return Ok(true);
    };

    if let Err(first_entry_error) = first_entry_result {
        return Err(DirectoryEmptinessScanErrorV2::UnableToReadDirectoryEntry {
            directory_path: directory_path.to_path_buf(),
            error: first_entry_error,
        });
    }

    Ok(false)
}
