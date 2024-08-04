use std::{
    cmp::Ordering,
    fs::Metadata,
    path::{Path, PathBuf},
};


use_enabled_fs_module!();

use crate::error::{DirectoryEmptinessScanErrorV2, DirectoryScanErrorV2};

pub(crate) mod collected;
mod iter;
pub use iter::*;




/// The maximum directory scan depth option.
///
/// Used primarily in [`DirectoryScanner`].
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


/// Options that influence [`DirectoryScanner`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectoryScanOptionsV2 {
    /// Whether to have the iterator yield the base directory
    /// as its first item or not.
    pub yield_base_directory: bool,

    /// The maximum directory scanning depth, see [`DirectoryScanDepthLimit`].
    pub maximum_scan_depth: DirectoryScanDepthLimit,

    /// If enabled, symlinks inside the scan tree will be followed,
    /// meaning yielded [`ScanEntry`] elements will have their paths
    /// resolved in case of a symlink.
    ///
    /// If a symlink cycle is detected inside the tree,
    /// an error is returned when it is encountered.
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
    /// [`next`]: BreadthFirstDirectoryIter::next
    pub follow_base_directory_symbolic_link: bool,
}

impl DirectoryScanOptionsV2 {
    #[inline]
    pub(crate) const fn should_track_ancestors(&self) -> bool {
        self.follow_symbolic_links
    }
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



/// Describes the depth of a scanned entry.
///
/// The depth is usually relative to a base directory (e.g. to the scan root).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanEntryDepth {
    /// The given entry is the base directory of the scan.
    BaseDirectory,

    /// The given entry is at `depth` levels under the base directory.
    AtDepth {
        /// Describes the depth of the scan entry.
        ///
        /// In this context, 0 means the entry is a direct descendant of the base directory,
        /// 1 means it is a grandchild, and so on.
        depth: usize,
    },
}

impl ScanEntryDepth {
    /// Returns a [`ScanEntryDepth`] that is one level deeper than the current one.
    fn plus_one_level(self) -> Self {
        match self {
            ScanEntryDepth::BaseDirectory => ScanEntryDepth::AtDepth { depth: 0 },
            ScanEntryDepth::AtDepth { depth } => ScanEntryDepth::AtDepth { depth: depth + 1 },
        }
    }
}

impl PartialOrd for ScanEntryDepth {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ScanEntryDepth::BaseDirectory, ScanEntryDepth::BaseDirectory) => Some(Ordering::Equal),
            (ScanEntryDepth::BaseDirectory, ScanEntryDepth::AtDepth { .. }) => Some(Ordering::Less),
            (ScanEntryDepth::AtDepth { .. }, ScanEntryDepth::BaseDirectory) => {
                Some(Ordering::Greater)
            }
            (
                ScanEntryDepth::AtDepth { depth: left_depth },
                ScanEntryDepth::AtDepth { depth: right_depth },
            ) => left_depth.partial_cmp(right_depth),
        }
    }
}


/// A directory scan entry.
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

    /// Returns the depth of the entry inside the scan tree.
    pub fn depth(&self) -> &ScanEntryDepth {
        &self.depth
    }

    /// Returns the [`Path`] of the scan entry.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the [`Metadata`] of the scan entry.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Consumes `self` and returns the owned path ([`PathBuf`]) of the scan entry.
    pub fn into_path(self) -> PathBuf {
        self.path
    }

    /// Consumes `self` and returns the [`Metadata`] of the scan entry.
    pub fn into_metadata(self) -> Metadata {
        self.metadata
    }

    /// Consumes `self` and returns the path ([`PathBuf`])
    /// and the [`Metadata`] of the scan entry.
    pub fn into_path_and_metadata(self) -> (PathBuf, Metadata) {
        (self.path, self.metadata)
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
/// [`walkdir`](https://docs.rs/walkdir) crate,
/// which this scanner has been inspired by.
pub struct DirectoryScanner {
    base_path: PathBuf,

    options: DirectoryScanOptionsV2,
}

impl DirectoryScanner {
    /// Initializes the directory scanner.
    ///
    /// This call will not interact with the filesystem yet. To turn this scanner struct into
    /// a breadth-first recursive iterator, call its [`into_iter`][`Self::into_iter`] method.
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
    type IntoIter = BreadthFirstDirectoryIter;
    type Item = Result<ScanEntry, DirectoryScanErrorV2>;

    fn into_iter(self) -> Self::IntoIter {
        BreadthFirstDirectoryIter::new(self.base_path, self.options)
    }
}



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
/// Permission and other errors will *not* be coerced into `false`,
/// but will instead raise a distinct error (see [`DirectoryEmptinessScanErrorV2`]).
pub fn is_directory_empty<P>(directory_path: P) -> Result<bool, DirectoryEmptinessScanErrorV2>
where
    P: AsRef<Path>,
{
    // TODO needs more tests!

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
