use std::{
    collections::VecDeque,
    fs::Metadata,
    path::{Path, PathBuf},
};

use_enabled_fs_module!();

use crate::{
    error::{
        DirectoryScanError,
        DirectoryScanErrorV2,
        DirectorySizeScanError,
        FileSizeError,
        IsDirectoryEmptyError,
    },
    file::file_size_in_bytes,
};



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


// TODO

// #[derive(Clone, PartialEq, Eq, Debug)]
// pub enum DirectoryScanIterationOrder {
//     DirectoriesFirst,
//     FilesFirst,
// }

// #[derive(Clone, PartialEq, Eq, Debug)]
// pub enum DirectoryScanTraversalMode {
//     BreadthFirst,
//     DepthFirst,
// }

/// Options that influence [`DirectoryScan`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryScanOptionsV2 {
    pub yield_base_directory: bool,

    /// The maximum directory scanning depth, see [`DirectoryScanDepthLimit`].
    /// TODO Not implemented yet.
    pub maximum_scan_depth: DirectoryScanDepthLimit,

    // pub iteration_order: DirectoryScanIterationOrder,
    // pub traversal_mode: DirectoryScanTraversalMode,
    /// TODO Not implemented yet.
    pub follow_symbolic_links: bool,

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


struct OpenDirectory {
    directory_path: PathBuf,

    directory_depth: ScanEntryDepth,

    iterator: fs::ReadDir,
}

struct PendingDirectory {
    directory_path: PathBuf,

    directory_depth: ScanEntryDepth,
}


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

macro_rules! try_some {
    ($expression:expr) => {
        match $expression {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        }
    };

    ($expression:expr, $error_mapper:expr) => {
        match $expression {
            Ok(value) => value,
            Err(error) => return Some(Err(error).map_err($error_mapper)),
        }
    };
}


struct NextEntryInfo {
    path: PathBuf,
    metadata: Metadata,
    followed_symlink: bool,
    depth: ScanEntryDepth,
}


pub struct DirectoryScannerPerDirectoryIter {
    base_directory: PathBuf,

    has_processed_base_directory: bool,

    has_scanned_base_directory: bool,

    options: DirectoryScanOptionsV2,

    currently_open_directory: Option<OpenDirectory>,

    pending_directory_stack: VecDeque<PendingDirectory>,
}

impl DirectoryScannerPerDirectoryIter {
    fn new<P>(base_directory: P, options: DirectoryScanOptionsV2) -> Self
    where
        P: Into<PathBuf>,
    {
        let base_directory: PathBuf = base_directory.into();

        Self {
            base_directory,
            has_processed_base_directory: false,
            has_scanned_base_directory: false,
            options,
            currently_open_directory: None,
            pending_directory_stack: VecDeque::new(),
        }
    }

    fn current_directory_handle_mut(
        &mut self,
    ) -> Result<Option<&mut OpenDirectory>, DirectoryScanErrorV2> {
        if self.currently_open_directory.is_some() {
            let handle = self
                .currently_open_directory
                .as_mut()
                .expect("currently_open_directory should be Some");

            return Ok(Some(handle));
        }

        self.open_next_directory_handle()
    }

    fn open_next_directory_handle(
        &mut self,
    ) -> Result<Option<&mut OpenDirectory>, DirectoryScanErrorV2> {
        if !self.has_scanned_base_directory {
            // We've just started, perhaps having just yielded the base directory.
            // As such, we should open the base directory.

            let base_dir_iterator = fs::read_dir(&self.base_directory).map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectory {
                    directory_path: self.base_directory.clone(),
                    error: io_error,
                }
            })?;

            let active_reader_entry = OpenDirectory {
                directory_path: self.base_directory.clone(),
                directory_depth: ScanEntryDepth::BaseDirectory,
                iterator: base_dir_iterator,
            };


            assert!(self.currently_open_directory.is_none());
            self.currently_open_directory = Some(active_reader_entry);

            self.has_scanned_base_directory = true;


            let handle_mut = self.currently_open_directory
                .as_mut()
                // PANIC SAFETY: We just `push`-ed onto the vector.
                .expect("currently_open_directory should be Some");

            return Ok(Some(handle_mut));
        }


        // The base directory has already been opened or read;
        // open one pending directory from the pending directory stack instead,
        // or return `None` if no pending directories are left.
        let Some(next_pending_directory) = self.pending_directory_stack.pop_front() else {
            return Ok(None);
        };


        let directory_iterator =
            fs::read_dir(&next_pending_directory.directory_path).map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectory {
                    directory_path: next_pending_directory.directory_path.clone(),
                    error: io_error,
                }
            })?;

        let active_reader_entry = OpenDirectory {
            directory_path: next_pending_directory.directory_path,
            directory_depth: next_pending_directory.directory_depth,
            iterator: directory_iterator,
        };

        // This will also drop the previously open directory.
        // It is up to the caller of this function to ensure this is wanted behaviour.
        self.currently_open_directory = Some(active_reader_entry);


        let handle_mut = self.currently_open_directory
            .as_mut()
            // PANIC SAFETY: We just `push`-ed onto the vector.
            .expect("currently_open_directory should be Some");

        Ok(Some(handle_mut))
    }

    fn close_current_directory_handle(&mut self) -> Result<(), ()> {
        self.currently_open_directory.take().map(|_| ()).ok_or(())
    }

    fn queue_directory_for_scanning(&mut self, directory_path: PathBuf, depth: ScanEntryDepth) {
        let pending_dir_entry = PendingDirectory {
            directory_path,
            directory_depth: depth,
        };
        self.pending_directory_stack.push_back(pending_dir_entry);
    }

    fn next_entry(&mut self) -> Result<Option<NextEntryInfo>, DirectoryScanErrorV2> {
        loop {
            let follow_symbolic_links = self.options.follow_symbolic_links;

            let Some(current_directory_iterator) = self.current_directory_handle_mut()? else {
                return Ok(None);
            };


            match current_directory_iterator.iterator.next() {
                Some(raw_entry_result) => {
                    let raw_entry = raw_entry_result.map_err(|io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                            directory_path: current_directory_iterator.directory_path.clone(),
                            error: io_error,
                        }
                    })?;

                    let raw_entry_metadata = raw_entry.metadata().map_err(|io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                            directory_path: current_directory_iterator.directory_path.clone(),
                            error: io_error,
                        }
                    })?;


                    let (raw_entry_path, raw_entry_metadata, raw_entry_followed_symlink) =
                        if follow_symbolic_links && raw_entry_metadata.is_symlink() {
                            let resolved_raw_entry_path =
                                fs::read_link(raw_entry.path()).map_err(|io_error| {
                                    DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                        directory_path: current_directory_iterator
                                            .directory_path
                                            .clone(),
                                        error: io_error,
                                    }
                                })?;

                            let raw_entry_metadata_followed =
                                fs::symlink_metadata(&resolved_raw_entry_path).map_err(
                                    |io_error| DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                        directory_path: current_directory_iterator
                                            .directory_path
                                            .clone(),
                                        error: io_error,
                                    },
                                )?;

                            (resolved_raw_entry_path, raw_entry_metadata_followed, true)
                        } else {
                            (raw_entry.path(), raw_entry_metadata, false)
                        };


                    return Ok(Some(NextEntryInfo {
                        path: raw_entry_path,
                        metadata: raw_entry_metadata,
                        followed_symlink: raw_entry_followed_symlink,
                        depth: current_directory_iterator.directory_depth.plus_one_level(),
                    }));
                }
                None => {
                    self.close_current_directory_handle()
                        // PANIC SAFETY: We just held a reference to an open directory,
                        // which means `close_current_directory_handle` will be able to remove it, 
                        // as it does exist.
                        .expect("at least one directory should be currently opened");

                    // The loop will restart and a new directory will be opened.
                    // If there are no further directories to scan, `None` will be returned from the iterator.
                }
            }
        }
    }
}



impl Iterator for DirectoryScannerPerDirectoryIter {
    type Item = Result<ScanEntry, DirectoryScanErrorV2>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_processed_base_directory {
            self.has_processed_base_directory = true;

            // Follow symlink if configured to do so.
            let base_directory_metadata =
                try_some!(fs::symlink_metadata(&self.base_directory), |io_error| {
                    DirectoryScanErrorV2::UnableToReadDirectory {
                        directory_path: self.base_directory.clone(),
                        error: io_error,
                    }
                });


            if !base_directory_metadata.is_symlink() && !base_directory_metadata.is_dir() {
                return Some(Err(DirectoryScanErrorV2::NotADirectory {
                    path: self.base_directory.clone(),
                }));
            }

            if base_directory_metadata.is_symlink() {
                if !self.options.yield_base_directory {
                    // Nothing no follow, nothing to yield - the iterator will have no elements.
                    return None;
                }

                let symlink_destination =
                    try_some!(fs::read_link(&self.base_directory), |io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectory {
                            directory_path: self.base_directory.clone(),
                            error: io_error,
                        }
                    });

                let symlink_destination_metadata =
                    try_some!(fs::symlink_metadata(&symlink_destination), |io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectory {
                            directory_path: self.base_directory.clone(),
                            error: io_error,
                        }
                    });

                if !symlink_destination_metadata.is_dir() {
                    return Some(Err(DirectoryScanErrorV2::NotADirectory {
                        path: self.base_directory.clone(),
                    }));
                }


                // We followed the symlink, and we should now update our iterator's base directory path.
                self.base_directory = symlink_destination;
            }


            if self.options.yield_base_directory {
                return Some(Ok(ScanEntry::new(
                    self.base_directory.clone(),
                    base_directory_metadata,
                    ScanEntryDepth::BaseDirectory,
                )));
            }
        }


        let next_entry = {
            let Some(next_entry_info) = try_some!(self.next_entry()) else {
                // No further entries, the iterator has concluded. Once this is reached,
                // all subsequent calls to `next` will also hit this branch, returning `None`.
                return None;
            };


            if next_entry_info.metadata.is_dir() {
                let ScanEntryDepth::AtDepth {
                    depth: current_dir_depth,
                } = next_entry_info.depth
                else {
                    // PANIC SAFETY: Only the base directory can be emitted with `ScanEntryDepth::BaseDirectory`,
                    // and the code flow ensures it's not in this branch.
                    panic!("expected the next entry's depth to be 0+, not base directory");
                };

                match self.options.maximum_scan_depth {
                    DirectoryScanDepthLimit::Unlimited => {
                        self.queue_directory_for_scanning(
                            next_entry_info.path.clone(),
                            next_entry_info.depth,
                        );
                    }
                    DirectoryScanDepthLimit::Limited { maximum_depth } => {
                        if current_dir_depth < maximum_depth {
                            self.queue_directory_for_scanning(
                                next_entry_info.path.clone(),
                                next_entry_info.depth,
                            );
                        }
                    }
                }
            }


            ScanEntry::new(next_entry_info.path, next_entry_info.metadata, next_entry_info.depth)
        };


        Some(Ok(next_entry))
    }
}



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
