use std::path::{Path, PathBuf};

use super::file::CapturedFileState;



pub trait AsPath {
    fn as_path(&self) -> &Path;
}


pub trait WithSubPath {
    fn sub_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>;
}



pub trait AssertablePath {
    /*
     * General assertions.
     */

    /// Asserts the underlying path points to an existing entry on the filesystem,
    /// be it a file, directory, symlink, etc.
    fn assert_exists(&self);

    /// Asserts the path does not exist on the filesystem.
    fn assert_not_exists(&self);


    /*
     * Directory-related assertions.
     */

    /// Asserts the path points to a directory, or a symlink to one.
    fn assert_is_directory(&self);

    /// Asserts the path points to a directory;
    /// a symlink to a directory is treated as a failure.
    fn assert_is_directory_and_not_symlink(&self);

    /// Asserts the path points to a directory, or a symlink to one,
    /// and does not contain any files or directories.
    fn assert_is_directory_and_empty(&self);

    /// Asserts the path points to a directory, or a symlink to one,
    /// and does contains at least one entry.
    fn assert_is_directory_and_not_empty(&self);

    /// Asserts contents of directory at `self` and `other_directory_path` perfectly match content-wise.
    ///
    /// Structure and exact file contents are compared (two-way).
    fn assert_is_directory_and_fully_matches_secondary_directory<P>(&self, other_directory_path: P)
    where
        P: AsPath + AssertablePath;

    /// Asserts contents of directory at `self` and `other_directory_path` perfectly match content-wise.
    ///
    /// Structure and exact file contents are compared (two-way).
    ///
    /// Additionally, the caller may specify if the comparison should require symlinks in the primary
    /// directory to also be symlinks to the same contents in the secondary directory
    /// (this is the behaviour for `strict_symlink_comparison = true`). If set to `false`,
    /// only the contents are compared.
    fn assert_is_directory_and_fully_matches_secondary_directory_with_options<P>(
        &self,
        secondary_directory_path: P,
        strict_symlink_comparison: bool,
    ) where
        P: AsPath + AssertablePath;

    /// Asserts contents of directory at `other_directory_path` are present in the one at `self`.
    ///
    /// Matches [`with-options`]`(other_directory_path, true)`
    ///
    /// Structure and exact file contents are compared (one-way).
    ///
    ///
    /// [`with-options`]: AssertablePath::assert_is_directory_and_has_contents_of_secondary_directory_with_options
    fn assert_is_directory_and_has_contents_of_secondary_directory<P>(
        &self,
        other_directory_path: P,
    ) where
        P: AsPath + AssertablePath;

    /// Asserts contents of directory at `other_directory_path` are present in the one at `self`.
    ///
    /// Structure and exact file contents are compared (one-way).
    ///
    /// Additionally, the caller may specify if the comparison should require symlinks in the primary
    /// directory to also be symlinks to the same contents in the secondary directory
    /// (this is the behaviour for `strict_symlink_comparison = true`). If set to `false`,
    /// only the contents are compared.
    fn assert_is_directory_and_has_contents_of_secondary_directory_with_options<P>(
        &self,
        secondary_directory_path: P,
        strict_symlink_comparison: bool,
    ) where
        P: AsPath + AssertablePath;

    /*
     * File-related assertions.
     */

    /// Asserts the path points to a file, or a symlink to one.
    fn assert_is_file(&self);

    /// Asserts the path points to a file;
    /// a symlink to a file is treated as a failure.
    fn assert_is_file_and_not_symlink(&self);



    /*
     * Symlink-related assertions.
     */

    /// Asserts the path points to a symlink.
    ///
    /// This method does not ensure that the link destination is valid.
    fn assert_is_any_symlink(&self);

    /// Asserts the path points to a symlink and returns its destination
    /// (which can be relative).
    ///
    /// This method does not ensure that the link destination is valid.
    fn assert_is_any_symlink_and_resolve_destination(&self) -> PathBuf;

    /// Asserts the path is a symlink and points to a valid destination.
    fn assert_is_any_valid_symlink(&self);

    /// Asserts the path is a symlink and points to an invalid destination
    /// (i.e. the link is broken).
    fn assert_is_any_broken_symlink(&self);

    /// Asserts the path is a symlink and points to an invalid destination
    /// (i.e. the link is broken) and returns its destination,
    /// which can be relative.
    fn assert_is_any_broken_symlink_and_read_destination(&self) -> PathBuf;

    /// Asserts the path points to a symlink to a directory.
    fn assert_is_valid_symlink_to_directory(&self);

    /// Asserts the path points to a symlink to a directory,
    /// and that the destination of the symlink matches the provided `expected_destination_path`.
    fn assert_is_valid_symlink_to_directory_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>;

    /// Asserts the path points to a symlink to a directory,
    /// and that the destination of the symlink matches the provided `expected_destination_path`.
    fn assert_is_broken_symlink_to_directory_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>;

    /// Asserts the path points to a symlink to a directory,
    /// and returns the symlink destination.
    fn assert_is_valid_symlink_to_directory_and_resolve_destination(&self) -> PathBuf;


    /// Asserts the path points to a symlink to a file.
    ///
    /// The symbolic link must point to a valid location.
    fn assert_is_valid_symlink_to_file(&self);

    /// Asserts the path points to a symlink to a file,
    /// and that the destination of the symlink matches the provided `expected_destination_path`.
    ///
    /// The symbolic link must point to a valid location.
    fn assert_is_valid_symlink_to_file_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>;

    /// Asserts the path points to a symlink to a file,
    /// and returns the symlink destination.
    ///
    /// The symbolic link must point to a valid location.
    fn assert_is_valid_symlink_to_file_and_resolve_destination(&self) -> PathBuf;
}



pub trait ManageablePath {
    /// Given a `destination_file_path`, this method will create a symlink at `self`
    /// that points to the destination file path.
    ///
    ///
    /// # Panic
    /// This will panic if the symlink cannot be created, or if the destination
    /// path is not a file.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn symlink_to_file<P>(&self, destination_file_path: P)
    where
        P: AsRef<Path>;

    /// Given a `destination_directory_path`, this method will create a symlink at `self`
    /// that points to the destination directory path.
    ///
    ///
    /// # Panic
    /// This will panic if the symlink cannot be created, or if the destination
    /// path is not a directory.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn symlink_to_directory<P>(&self, destination_directory_path: P)
    where
        P: AsRef<Path>;


    /// This method will create an empty directory at `self`.
    /// Additionally, if the parent directory is missing, it will be created.
    ///
    ///
    /// # Panic
    /// This will panic if the path exists, or if directory cannot be created.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn assert_not_exists_and_create_empty_directory(&self);

    /// This method will create an empty file at `self`,
    /// if a file doesn't already exist. It will not truncate the
    /// file if it exists.
    ///
    /// Additionally, if the parent directory is missing, it will be created.
    ///
    ///
    /// # Panic
    /// This will panic if the path exists, or if file or parent directory
    /// cannot be created.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn assert_not_exists_and_create_empty_file(&self);

    /// Returns the size of the file at `self`, in bytes.
    fn size_in_bytes(&self) -> u64;

    /// Asserts the path at `self` points to a file,
    /// after which the file is removed.
    ///
    ///
    /// # Panic
    /// This will panic if the path does not point to a file,
    /// or if file removal fails.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn assert_is_file_and_remove(&self);

    /// Asserts the path at `self` points to an empty directory
    /// (and not a symlink to one), and removes the directory.
    ///
    ///
    /// # Panic
    /// This will panic if the path does not point to a directory,
    /// if the path points to a symlink, or if deletion fails.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn assert_is_empty_directory_and_remove(&self);

    /// Asserts the path at `self` is a symlink, and removes it.
    ///
    ///
    /// # Panic
    /// This will panic if the path does not point to a symlink,
    /// or if deletion fails.
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    fn assert_is_symlink_and_remove(&self);
}



pub trait CaptureableFilePath: AsPath {
    /// Explicitly creates a snapshot of the given file path's state.
    /// This includes information about whether the file exists, and additionally, its content.
    ///
    /// After capture, you may use e.g. [`CapturedFileState::assert_unchanged`]
    /// to assert that the current state of the file has not deviated from this snapshot.
    ///
    /// **Important: if you wish to compare a file with its state at test harness initialization,
    /// you do not need to create a manual snapshot! Each file in the test harness is automatically
    /// captured at the harness' [`initialize`] call. See methods in [`AssertableInitialFileCapture`],
    /// e.g. [`assert_unchanged_from_initial_state`]**
    ///
    ///
    /// # Panic
    /// Just like [`CapturedFileState::new_with_content_capture`]
    /// and [`FileState::capture_from_file_path`],
    /// this will panic if the provided path exists, but is not a file,
    /// or if the path cannot be accessed (due to permission or other IO errors).
    ///
    /// This is fine only because we *should fail on errors anyway*,
    /// since this is part of `fs-more`'s testing harness.
    ///
    ///
    /// [`AssertableInitialFileCapture`]: crate::trees::AssertableInitialFileCapture
    /// [`assert_unchanged_from_initial_state`]: crate::trees::AssertableInitialFileCapture::assert_unchanged_from_initial_state
    /// [`initialize`]: crate::trees::FileSystemHarness::initialize
    /// [`FileState::capture_from_file_path`]: crate::assertable::file::FileState::capture_from_file_path
    fn capture_with_content(&self) -> CapturedFileState {
        CapturedFileState::new_with_content_capture(self.as_path())
    }
}
