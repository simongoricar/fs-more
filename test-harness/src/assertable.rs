use std::{
    borrow::Cow,
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


#[derive(Error, Debug)]
pub enum AssertableFilePathError {
    #[error("provided file path doesn't exist")]
    NotFound,

    #[error("provided file path exists, but is not a file")]
    NotAFile,
    // TODO
    #[error("other std::io::Error: {0}")]
    OtherIoError(#[from] std::io::Error),
}

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

    /// Returns this assertable root directory's filesystem path as a [`Path`][std::path::Path] reference.
    pub fn path(&self) -> &Path {
        &self.directory_path
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

    /// Returns this assertable directory's filesystem path as a [`Path`][std::path::Path] reference.
    pub fn path(&self) -> &Path {
        &self.directory_path
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
/// - [`from_path_pure`][Self::from_path_pure] and
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
    /// [`FilesystemTreeHarness`](../../fs_more_test_harness_derive/derive.FilesystemTreeHarness.html)
    /// procedural macro - as such, ignore this method in your own uses.
    pub fn from_child_path(
        child_path: ChildPath,
        original_contents: &'static [u8],
    ) -> Self {
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
    pub fn from_path_pure<P>(file_path: P) -> Self
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
    pub fn from_path_with_captured_content<P>(
        file_path: P,
    ) -> Result<Self, AssertableFilePathError>
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
    /// Just like [`from_path_pure`][Self::from_path_pure],
    /// this initialization method *does not interact with the filesystem at all*.
    ///
    /// Therefore, the "expected" contents of the file (see the [`expected_content`][Self::expected_content] method)
    /// will be set to what you pass in with the `expected_content` parameter.
    pub fn from_path_with_expected_content<P, C>(
        path: P,
        expected_content: C,
    ) -> Self
    where
        P: Into<PathBuf>,
        C: Into<Cow<'static, [u8]>>,
    {
        Self {
            file_path: path.into(),
            expected_file_content: Some(expected_content.into()),
        }
    }

    /// Returns this assertable file's filesystem path as a [`Path`][std::path::Path] reference.
    pub fn path(&self) -> &Path {
        &self.file_path
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
    /// or [`from_path_pure`][Self::from_path_pure]
    /// methods to initialize [`Self`]).
    pub fn expected_content_unchecked(&self) -> &[u8] {
        self.expected_file_content
            .as_ref()
            .expect("Expected file content is unknown.")
    }

    /// Assert this file exists.
    pub fn assert_exists(&self) {
        assert!(self.file_path.exists() && self.file_path.is_file());
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
    /// or [`from_path_pure`][Self::from_path_pure]
    /// methods to initialize [`Self`]).
    pub fn assert_content_unchanged(&self) {
        let Some(expected_contents) = self.expected_file_content.as_ref() else {
            panic!(
                "Expected file contents are unknown."
            )
        };

        let actual_content =
            std::fs::read(self.path()).expect("Failed to read file.");

        let contents_match = actual_content.eq(expected_contents.as_ref());

        if !contents_match {
            let real_content_described =
                display_bytes_unless_large(&actual_content, 32);

            let expected_content_described =
                display_bytes_unless_large(expected_contents, 32);

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
    pub fn assert_content_matches_expected_value_of_assertable(
        &self,
        other: &Self,
    ) {
        let other_expected_content = other
            .expected_content()
            .expect("other's expected_content is unknown");

        let self_content =
            std::fs::read(&self.file_path).unwrap_or_else(|err| {
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
