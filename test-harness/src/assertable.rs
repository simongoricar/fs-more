use std::path::{Path, PathBuf};

use assert_fs::{fixture::ChildPath, TempDir};

use crate::{assert_file_bytes_match, assert_file_string_match};

/// A root directory path abstraction for testing purposes.
///
/// Allows the user to assert certain things, such as the root directory existing or not.
///
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see the [`new`][Self::new] initialization method.
pub struct AssertableRootPath {
    #[allow(dead_code)]
    root: TempDir,

    directory_path: PathBuf,
}

impl AssertableRootPath {
    /// Initialize a new assertable root directory path from the provided
    /// [`assert_fs::TempDir`](../../assert_fs/fixture/struct.TempDir.html).
    pub fn new(root: TempDir) -> Self {
        let directory_path = root.path().to_path_buf();

        Self {
            root,
            directory_path,
        }
    }

    /// Assert the directory exists.
    pub fn assert_exists(&self) {
        assert!(self.directory_path.exists() && self.directory_path.is_dir());
    }

    /// Assert the directory does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.directory_path.exists());
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
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see the [`from_path`][Self::from_path] initialization method.
pub struct AssertableDirectoryPath {
    /// Directory path.
    directory_path: PathBuf,
}

impl AssertableDirectoryPath {
    /// *Warning:* this initialization method is intended for the
    /// [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
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

    /// Assert the directory exists.
    pub fn assert_exists(&self) {
        assert!(self.directory_path.exists() && self.directory_path.is_dir());
    }

    /// Assert the directory does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.directory_path.exists());
    }
}


/// A file path abstraction for testing purposes.
///
/// Allows the user to assert certain things, such as the file existing or not, or its contents.
///
/// Mainly intended to be used with the [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
/// macro, but can also be used standalone, see these initialization methods:
/// - [`from_path_as_empty`][Self::from_path_as_empty] and
/// - [`from_path_with_expected_content`][Self::from_path_with_expected_content].
pub struct AssertableFilePath {
    /// File path.
    file_path: PathBuf,

    /// The expected file contents as a static `u8` slice.
    /// This is used by the [`assert_content_unchanged`][Self::assert_content_unchanged]
    /// method to assert a file is unchanged.
    original_contents: &'static [u8],
}

impl AssertableFilePath {
    /// *Warning:* this initialization method is intended for the
    /// [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
    /// procedural macro - as such, ignore this method in your own uses.
    pub fn from_child_path(
        child_path: ChildPath,
        original_contents: &'static [u8],
    ) -> Self {
        Self {
            file_path: child_path.path().to_path_buf(),
            original_contents,
        }
    }

    /// Initialize a new assertable file path by providing just a file path
    /// (this initialization method does not create the file or do anything with it).
    ///
    /// The expected contents of the file will be set to an empty vector (i.e. an empty file).
    pub fn from_path_as_empty<P>(path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            file_path: path.into(),
            original_contents: &[],
        }
    }

    /// Initialize a new assertable file path by providing just a file path
    /// (this initialization method does not create the file or do anything with it).
    ///
    /// Also provide the contents you expect it to have (the contents aren't compared on initialization,
    /// see the [`assert_content_unchanged`][Self::assert_content_unchanged]
    /// method for the use of `expected_content`).
    pub fn from_path_with_expected_content<P>(
        path: P,
        expected_content: &'static [u8],
    ) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            file_path: path.into(),
            original_contents: expected_content,
        }
    }

    /// Returns this assertable file's filesystem path as a [`Path`][std::path::Path] reference.
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Assert this file exists.
    pub fn assert_exists(&self) {
        assert!(self.file_path.exists() && self.file_path.is_file());
    }

    /// Assert this file does not exist.
    pub fn assert_not_exists(&self) {
        assert!(!self.file_path.exists());
    }

    /// Assert the file's contents are unchanged (see the `original_contents` parameter in the
    /// [`from_path_with_expected_content`][Self::from_path_with_expected_content] function).
    pub fn assert_content_unchanged(&self) {
        assert_file_bytes_match!(&self.file_path, self.original_contents);
    }

    /// Assert a file's contents match another file.
    ///
    /// You can provide anything that implements `AsRef<Path>`, *which includes [`Self`]*.
    pub fn assert_content_matches_another_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let self_content =
            std::fs::read(&self.file_path).unwrap_or_else(|err| {
                panic!(
                    "Failed to read file path \"{}\": {}",
                    self.file_path.display(),
                    err
                )
            });

        let other_content =
            std::fs::read(other.as_ref()).unwrap_or_else(|err| {
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

impl AsRef<Path> for AssertableFilePath {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}
