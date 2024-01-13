use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use assert_fs::{fixture::ChildPath, TempDir};
use thiserror::Error;

use crate::{assert_file_bytes_match, assert_file_string_match};


fn display_bytes_unless_large(bytes: &[u8], maximum_size: usize) -> String {
    if bytes.len() > maximum_size {
        format!("[{} bytes]", bytes.len())
    } else {
        format!("\"{}\"", String::from_utf8_lossy(bytes))
    }
}

fn assert_directory_contents_match_other_directory<P1, P2>(
    self_directory_path: P1,
    other_directory_path: P2,
) where
    P1: Into<PathBuf>,
    P2: Into<PathBuf>,
{
    let self_directory_path: PathBuf = self_directory_path.into();
    let other_directory_path: PathBuf = other_directory_path.into();

    // The process is as follows:
    // - we construct a scan queue containing directory paths (`self_directory_path` or subdirectories),
    // - we process the queue (FIFO), comparing files and directories (and ignoring symbolic links or any file metadata).
    let mut directory_scan_queue = vec![self_directory_path.clone()];

    while let Some(next_directory_to_scan) = directory_scan_queue.pop() {
        let directory_scan = std::fs::read_dir(&next_directory_to_scan).unwrap_or_else(|error| {
            panic!(
                "failed to read contents of directory {}: {}",
                next_directory_to_scan.display(),
                error
            );
        });

        for entry in directory_scan {
            let entry = entry.expect("failed to get next directory entry");
            let entry_path = entry.path();

            let entry_type = entry.file_type().expect("failed to get entry's file type");
            let entry_type_str = if entry_type.is_dir() {
                "directory"
            } else if entry_type.is_file() {
                "file"
            } else if entry_type.is_symlink() {
                "symbolic link"
            } else {
                "unknown"
            };


            let subpath_of_self = entry_path
                .strip_prefix(&self_directory_path)
                .expect("scanned path should be a subdirectory of the base `self_directory_path`");

            let expected_path_on_other = other_directory_path.join(subpath_of_self);


            if entry_type.is_file() {
                assert!(
                    expected_path_on_other.exists(),
                    "directory contents do not match: \
                    file {} exists on `self`, but not on `other` \
                    (expected {} to be a file)",
                    subpath_of_self.display(),
                    expected_path_on_other.display()
                );

                assert!(
                    expected_path_on_other.is_file(),
                    "directory contents do not match: \
                    {} is a file on `self`, but is a {} on `other` instead \
                    (expected {} to be a file)",
                    subpath_of_self.display(),
                    entry_type_str,
                    expected_path_on_other.display()
                );

                // First, do a naive file size check and fail early if they don't match.
                let entry_file_size_bytes = entry
                    .metadata()
                    .unwrap_or_else(|error| {
                        panic!(
                            "failed to read metadata for file {} (on `self`): {}",
                            entry_path.display(),
                            error
                        )
                    })
                    .len();

                let other_file_size_bytes = expected_path_on_other
                    .metadata()
                    .unwrap_or_else(|error| {
                        panic!(
                            "failed to read metadata for file {} (on `other`): {}",
                            expected_path_on_other.display(),
                            error
                        )
                    })
                    .len();

                assert_eq!(
                    entry_file_size_bytes,
                    other_file_size_bytes,
                    "directory contents do not match: \
                    file {} is of different sizes on `self` and `other`",
                    subpath_of_self.display()
                );

                // If file sizes match, compare the contents.
                const BUFFER_SIZE: usize = 1024 * 16;

                let entry_file = {
                    let file = OpenOptions::new()
                        .read(true)
                        .open(&entry_path)
                        .unwrap_or_else(|error| {
                            panic!(
                                "failed to open file {} for reading (on `self`): {}",
                                entry_path.display(),
                                error
                            )
                        });

                    BufReader::with_capacity(BUFFER_SIZE, file)
                };

                let other_file = {
                    let file = OpenOptions::new()
                        .read(true)
                        .open(&expected_path_on_other)
                        .unwrap_or_else(|error| {
                            panic!(
                                "failed to open file {} for reading (on `other`): {}",
                                expected_path_on_other.display(),
                                error
                            )
                        });

                    BufReader::with_capacity(BUFFER_SIZE, file)
                };

                // FIXME This is only used for testing anyway, but maybe find a better way than byte-by-byte comparisons?
                for (byte_index, (entry_file_byte, other_file_byte)) in
                    entry_file.bytes().zip(other_file.bytes()).enumerate()
                {
                    let entry_file_byte = entry_file_byte.unwrap_or_else(|error| {
                        panic!(
                            "failed to read byte from file {} (on `other`): {}",
                            entry_path.display(),
                            error
                        );
                    });

                    let other_file_byte = other_file_byte.unwrap_or_else(|error| {
                        panic!(
                            "failed to read byte from file {} (on `self`): {}",
                            expected_path_on_other.display(),
                            error
                        );
                    });

                    if entry_file_byte != other_file_byte {
                        panic!(
                            "directory contents do not match: \
                            contents of file {} are not the same, \
                            byte {} is {} on `self`, but {} on `other`",
                            subpath_of_self.display(),
                            byte_index,
                            entry_file_byte,
                            other_file_byte
                        );
                    }
                }
            } else if entry_type.is_dir() {
                assert!(
                    expected_path_on_other.exists(),
                    "directory contents do not match: \
                    directory {} exists on `self`, but not on `other` \
                    (expected {} to be a directory)",
                    subpath_of_self.display(),
                    expected_path_on_other.display()
                );

                assert!(
                    expected_path_on_other.is_dir(),
                    "directory contents do not match: \
                    {} is a directory on `self`, but is a {} on `self` instead \
                    (expected {} to be a directory)",
                    subpath_of_self.display(),
                    entry_type_str,
                    expected_path_on_other.display()
                );

                // Queue scanning of the directory's contents by putting it into our scan queue.
                directory_scan_queue.push(entry_path);
            }
        }
    }
}



#[derive(Error, Debug)]
pub enum AssertableFilePathError {
    #[error("provided file path doesn't exist")]
    NotFound,

    #[error("provided file path exists, but is not a file")]
    NotAFile,

    #[error("unable to compute parent directory")]
    NoParentDirectory,

    #[error("other std::io::Error: {error}")]
    OtherIoError {
        #[from]
        error: std::io::Error,
    },
}

/// A root directory path abstraction for testing purposes.
///
/// Allows the user to assert certain things, such as the root directory existing or not.
///
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_macros/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see the [`new`][Self::new] initialization method.
pub struct AssertableRootDirectory {
    #[allow(dead_code)]
    root: TempDir,

    directory_path: PathBuf,
}

impl AssertableRootDirectory {
    /// Initialize a new assertable root directory path from the provided
    /// [`assert_fs::TempDir`](../../assert_fs/fixture/struct.TempDir.html).
    pub fn new(root: TempDir) -> Self {
        let directory_path = root.path().to_path_buf();

        Self {
            root,
            directory_path,
        }
    }

    /// Returns this assertable root directory's filesystem path as a [`Path`] reference.
    pub fn path(&self) -> &Path {
        &self.directory_path
    }

    /// Returns a child path (subpath) of this root path.
    ///
    /// ### Example
    /// ```rust
    /// # use fs_more_test_harness::assertable::AssertableRootDirectory;
    /// # use std::path::PathBuf;
    /// let temporary_dir = assert_fs::TempDir::new()
    ///     .expect("failed to create a temporary directory for testing");
    ///
    /// let assertable_root_dir = AssertableRootDirectory::new(temporary_dir);
    ///
    /// assert_eq!(
    ///     assertable_root_dir.child_path("foo"),
    ///     assertable_root_dir.path().join("foo")
    /// );
    /// ```
    pub fn child_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.path().join(sub_path)
    }

    /// Assert the directory exists.
    pub fn assert_exists(&self) {
        assert!(self.directory_path.exists() && self.directory_path.is_dir());
    }

    /// Assert the directory does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.directory_path.exists());
    }

    /// Assert the directory is completely empty.
    pub fn assert_is_empty(&self) {
        let directory_scan =
            std::fs::read_dir(self.path()).expect("failed to read contents of directory");

        assert_eq!(directory_scan.count(), 0);
    }

    /// Assert the directory is not completely empty.
    pub fn assert_is_not_empty(&self) {
        let directory_scan =
            std::fs::read_dir(self.path()).expect("failed to read contents of directory");

        assert!(directory_scan.count() > 0);
    }

    /// Assert contents of directory `self` and `other_directory_path` perfectly match.
    /// Structure and exact file contents are compared, but **symlinks and metadata are ignored**.
    pub fn assert_directory_contents_match_directory<P>(&self, other_directory_path: P)
    where
        P: Into<PathBuf>,
    {
        assert_directory_contents_match_other_directory(self.path(), other_directory_path);
    }

    /// Consume `self` and return the inner [`assert_fs::TempDir`](../../assert_fs/fixture/struct.TempDir.html).
    pub fn into_temp_dir(self) -> TempDir {
        self.root
    }
}


/// A directory path abstraction for testing purposes.
///
/// Allows the user to assert certain things, such as the directory existing or not.
///
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_macros/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see the [`from_path`][Self::from_path] initialization method.
pub struct AssertableDirectoryPath {
    /// Directory path.
    directory_path: PathBuf,
}

impl AssertableDirectoryPath {
    /// *Warning:* this initialization method is intended for the
    /// [`FilesystemTreeHarness`](../../fs_more_test_harness_macros/derive.FilesystemTreeHarness.html)
    /// procedural macro - as such, ignore this method in your own uses.
    pub fn from_child_path(child_path: ChildPath) -> Self {
        Self {
            directory_path: child_path.path().to_path_buf(),
        }
    }

    /// Initialize a new assertable file directory.
    pub fn from_path<P>(directory_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            directory_path: directory_path.into(),
        }
    }

    /// Returns this assertable directory's filesystem path as a [`Path`] reference.
    ///
    /// ### Example
    /// ```rust
    /// # use fs_more_test_harness::assertable::AssertableDirectoryPath;
    /// # use std::path::Path;
    /// let some_path = Path::new("/foo/bar");
    /// let assertable_directory_path = AssertableDirectoryPath::from_path(some_path);
    ///
    /// assert_eq!(
    ///     some_path,
    ///     assertable_directory_path.path(),
    /// );
    /// ```
    pub fn path(&self) -> &Path {
        &self.directory_path
    }

    /// Returns a child path (subpath) of this directory path.
    ///
    /// ### Example
    /// ```rust
    /// # use fs_more_test_harness::assertable::AssertableRootDirectory;
    /// # use std::path::PathBuf;
    /// let temporary_dir = assert_fs::TempDir::new()
    ///     .expect("failed to create a temporary directory for testing");
    ///
    /// let assertable_root_dir = AssertableRootDirectory::new(temporary_dir);
    ///
    /// assert_eq!(
    ///     assertable_root_dir.child_path("foo.txt"),
    ///     assertable_root_dir.path().join("foo.txt")
    /// );
    /// ```
    pub fn child_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.path().join(sub_path)
    }

    /// Creates a symbolic link to a target directory.
    pub fn symlink_to_directory<P>(&self, target_path: P) -> Result<(), AssertableFilePathError>
    where
        P: AsRef<Path>,
    {
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(target_path.as_ref(), &self.directory_path)
                .map_err(|error| AssertableFilePathError::OtherIoError { error })?;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target_path.as_ref(), &self.file_path)
                .map_err(|error| AssertableFilePathError::OtherIoError { error })?;
        }

        #[cfg(not(any(windows, unix)))]
        {
            compile_error!(
                "fs-more supports only the following values of target_family: unix and windows \
                (notably, wasm is unsupported)."
            );
        }

        Ok(())
    }

    /// Assert the directory exists.
    pub fn assert_exists(&self) {
        assert!(self.directory_path.exists() && self.directory_path.is_dir());
    }

    /// Assert the directory does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.directory_path.exists());
    }

    /// Asserts that this path leads to a directory which is not a symlink.
    pub fn assert_is_directory(&self) {
        assert!(self.directory_path.is_dir() && !self.directory_path.is_symlink());
    }

    /// Asserts that this path leads to a symbolic link to a directory.
    pub fn assert_is_symlink_to_directory(&self) {
        assert!(self.directory_path.is_symlink() && self.directory_path.is_dir());
    }

    /// Asserts that this path leads to a symbolic link (either a file or directory).
    pub fn assert_is_symlink(&self) {
        assert!(self.directory_path.is_symlink());
    }

    /// Assert the directory is completely empty.
    pub fn assert_is_empty(&self) {
        let directory_scan =
            std::fs::read_dir(self.path()).expect("failed to read contents of directory");

        assert_eq!(directory_scan.count(), 0);
    }

    /// Assert the directory is not completely empty.
    pub fn assert_is_not_empty(&self) {
        let directory_scan =
            std::fs::read_dir(self.path()).expect("failed to read contents of directory");

        assert!(directory_scan.count() > 0);
    }

    /// Assert contents of directory `self` and `other_directory_path` perfectly match.
    /// Structure and exact file contents are compared, but **symlinks and metadata are ignored**.
    pub fn assert_directory_contents_match_directory<P>(&self, other_directory_path: P)
    where
        P: Into<PathBuf>,
    {
        assert_directory_contents_match_other_directory(self.path(), other_directory_path);
    }
}


/// A file path abstraction for testing purposes.
///
/// Allows the user to assert certain things, such as the file existing or not, or its contents.
///
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_macros/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see these initialization methods:
/// - [`from_path`][Self::from_path] and
/// - [`from_path_with_expected_content`][Self::from_path_with_expected_content].
pub struct AssertableFilePath {
    /// File path.
    file_path: PathBuf,

    /// The "expected" file content as a static `u8` slice.
    ///
    /// "Expected" in this content means either:
    /// - unknown - indicated by `None`,
    /// - empty file - indicated by `Some(empty slice)`,
    /// - specific contents - indicated by `Some(byte contents)`.
    ///
    /// This is influenced by the choice of initialization method:
    /// [`from_path_pure`][Self::from_path_pure] will set this to `None`,
    /// indicating we don't know anything about content.
    /// [`from_path_with_capture`][Self::from_path_with_capture], however, will
    /// read the file and save a snapshot of the contents.
    ///
    /// This is used by the [`assert_content_unchanged`][Self::assert_content_unchanged]
    /// method to assert a file is unchanged.
    expected_file_content: Option<Cow<'static, [u8]>>,
}

impl AssertableFilePath {
    /// *Warning:* this initialization method is intended for the
    /// [`FilesystemTreeHarness`](../../fs_more_test_harness_macros/derive.FilesystemTreeHarness.html)
    /// procedural macro - as such, ignore this method in your own uses.
    pub fn from_child_path(child_path: ChildPath, original_contents: &'static [u8]) -> Self {
        Self {
            file_path: child_path.path().to_path_buf(),
            expected_file_content: Some(Cow::Borrowed(original_contents)),
        }
    }

    /// Initialize a new assertable file path by providing a file path.
    ///
    /// This initialization method *does not interact with the filesystem at all*.
    /// Therefore, the "expected" contents of the file (see the [`expected_content`][Self::expected_content] method)
    /// will be set to `None` (i.e. unknown).
    pub fn from_path<P>(file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            file_path: file_path.into(),
            expected_file_content: None,
        }
    }

    /// Initialize a new asssertable file path by providing a file path.
    /// **The file must exist.**
    ///
    /// This initialization method *interacts with the filesystem:* it creates
    /// a snapshot of the file contents (what we otherwise call the "expected content" here).
    /// When you later call [`assert_content_unchanged`][Self::assert_content_unchanged],
    /// you'll be able to assert whether the file changed since initialization.
    pub fn from_path_with_captured_content<P>(file_path: P) -> Result<Self, AssertableFilePathError>
    where
        P: Into<PathBuf>,
    {
        let file_path: PathBuf = file_path.into();

        if !file_path.exists() {
            return Err(AssertableFilePathError::NotFound);
        }
        if !file_path.is_file() {
            return Err(AssertableFilePathError::NotAFile);
        }

        let current_contents = std::fs::read(&file_path)?;

        Ok(Self {
            file_path,
            expected_file_content: Some(Cow::Owned(current_contents)),
        })
    }

    /// Initialize a new assertable file path by providing just a file path.
    ///
    /// Just like [`from_path`][Self::from_path],
    /// this initialization method *does not interact with the filesystem at all*.
    ///
    /// Therefore, the "expected" contents of the file (see the [`expected_content`][Self::expected_content] method)
    /// will be set to what you pass in with the `expected_content` parameter.
    pub fn from_path_with_expected_content<P, C>(path: P, expected_content: C) -> Self
    where
        P: Into<PathBuf>,
        C: Into<Cow<'static, [u8]>>,
    {
        Self {
            file_path: path.into(),
            expected_file_content: Some(expected_content.into()),
        }
    }

    /// Returns this assertable file's filesystem path as a [`Path`] reference.
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Ensures the parent directory of this file exists (i.e. creates the parent directory if it doesn't exist).
    pub fn ensure_parent_directory_exists(&self) -> Result<(), AssertableFilePathError> {
        let parent_directory = self
            .file_path
            .parent()
            .ok_or_else(|| AssertableFilePathError::NoParentDirectory)?;

        std::fs::create_dir_all(parent_directory)
            .map_err(|error| AssertableFilePathError::OtherIoError { error })
    }

    /// Creates an empty file.
    pub fn touch(&self) -> Result<(), AssertableFilePathError> {
        self.ensure_parent_directory_exists()?;

        let file = std::fs::File::create(&self.file_path)
            .map_err(|error| AssertableFilePathError::OtherIoError { error })?;
        drop(file);

        Ok(())
    }

    /// Creates a symbolic link to a target file.
    pub fn symlink_to_file<P>(&self, target_path: P) -> Result<(), AssertableFilePathError>
    where
        P: AsRef<Path>,
    {
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target_path.as_ref(), &self.file_path)
                .map_err(|error| AssertableFilePathError::OtherIoError { error })?;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target_path.as_ref(), &self.file_path)
                .map_err(|error| AssertableFilePathError::OtherIoError { error })?;
        }

        #[cfg(not(any(windows, unix)))]
        {
            compile_error!(
                "fs-more supports only the following values of target_family: unix and windows \
                (notably, wasm is unsupported)."
            );
        }

        Ok(())
    }


    /// Returns the file size in bytes. If the file is a symbolic link,
    /// this function returns the size of the file the symbolic link points to.
    pub fn file_size_in_bytes(&self) -> Result<u64, AssertableFilePathError> {
        let file_metadata = self
            .file_path
            .metadata()
            .map_err(|error| AssertableFilePathError::OtherIoError { error })?;

        Ok(file_metadata.len())
    }

    /// Sets the content the [`assert_content_unchanged`][Self::assert_content_unchanged]
    /// method expects when asserting whether the file has changed.
    pub fn set_expected_content<C>(&mut self, contents: C)
    where
        C: Into<Cow<'static, [u8]>>,
    {
        self.expected_file_content = Some(contents.into());
    }

    /// If set, returns a reference to the expected file contents (as a slice of bytes).
    pub fn expected_content(&self) -> Option<&[u8]> {
        self.expected_file_content
            .as_ref()
            .map(|content| content.as_ref())
    }

    /// Returns a reference to the expected file contents (as a slice of bytes).
    ///
    /// ## Panics
    /// This will panic if the expected contents are unknown
    /// (e.g. when using the [`from_path_with_expected_content`][Self::from_path_with_expected_content]
    /// or [`from_path`][Self::from_path]
    /// methods to initialize [`Self`]).
    pub fn expected_content_unchecked(&self) -> &[u8] {
        self.expected_file_content
            .as_ref()
            .expect("Expected file content is unknown.")
    }

    /// Asserts that this path leads to a file which is not a symlink.
    pub fn assert_is_file(&self) {
        assert!(self.file_path.is_file() && !self.file_path.is_symlink());
    }

    /// Asserts that this path leads to a symbolic link to a file.
    pub fn assert_is_symlink_to_file(&self) {
        assert!(self.file_path.is_symlink() && self.file_path.is_file());
    }

    /// Asserts that this path leads to a symbolic link (either a file or directory).
    pub fn assert_is_symlink(&self) {
        assert!(self.file_path.is_symlink());
    }

    /// Assert this file exists.
    pub fn assert_exists(&self) {
        assert!(self.file_path.exists());
    }

    /// Assert this file does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.file_path.exists());
    }

    /// Assert the file's contents are unchanged (see the `expected_content` parameter in the
    /// [`from_path_with_expected_content`][Self::from_path_with_expected_content] function).
    ///
    /// ## Panics
    /// This will also panic if the expected contents are unknown
    /// (e.g. when using the [`from_path_with_expected_content`][Self::from_path_with_expected_content]
    /// or [`from_path`][Self::from_path]
    /// methods to initialize [`Self`]).
    pub fn assert_content_unchanged(&self) {
        let Some(expected_contents) = self.expected_file_content.as_ref() else {
            panic!("Expected file contents are unknown.")
        };

        let actual_content = std::fs::read(self.path()).expect("Failed to read file.");

        let contents_match = actual_content.eq(expected_contents.as_ref());

        if !contents_match {
            let real_content_described = display_bytes_unless_large(&actual_content, 32);

            let expected_content_described = display_bytes_unless_large(expected_contents, 32);

            panic!(
                "File contents do not match: \n  {} (expected) \n    vs\n  {} (actual).",
                expected_content_described, real_content_described,
            )
        }
    }

    /// Assert a file's contents *match the* **expected** *contents of another
    /// [`Self`]*.
    ///
    /// ## Panics
    /// This method also panics if `other`'s expected content is unknown
    /// (i.e. when its [`expected_content`][Self::expected_content] returns `None`).
    pub fn assert_content_matches_expected_value_of_assertable(&self, other: &Self) {
        let other_expected_content = other
            .expected_content()
            .expect("other's expected_content is unknown");

        let self_content = std::fs::read(&self.file_path).unwrap_or_else(|err| {
            panic!(
                "Failed to read file path \"{}\": {}",
                self.file_path.display(),
                err
            )
        });

        assert_eq!(self_content, other_expected_content);
    }

    /// Assert a file's contents match another file.
    ///
    /// You can provide anything that implements `AsRef<Path>`.
    pub fn assert_content_matches_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let self_content = std::fs::read(&self.file_path).unwrap_or_else(|err| {
            panic!(
                "Failed to read file path \"{}\": {}",
                self.file_path.display(),
                err
            )
        });

        let other_content = std::fs::read(other.as_ref()).unwrap_or_else(|err| {
            panic!(
                "Failed to read file path \"{}\": {}",
                self.file_path.display(),
                err
            )
        });

        assert_eq!(self_content, other_content);
    }

    /// Assert a file's contents match a `&str`.
    pub fn assert_content_matches_str<C>(&self, content: C)
    where
        C: AsRef<str>,
    {
        assert_file_string_match!(&self.file_path, content.as_ref());
    }

    /// Assert a file's contents match a `[u8]`.
    pub fn assert_content_matches_bytes<C>(&self, content: C)
    where
        C: AsRef<[u8]>,
    {
        assert_file_bytes_match!(&self.file_path, content.as_ref());
    }
}
