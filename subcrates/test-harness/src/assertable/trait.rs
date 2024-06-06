use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    dir_comparison::{
        assert_primary_directory_precisely_contains_secondary_directory,
        DirectoryComparisonOptions,
        PathType,
    },
    file_capture::CapturedFileState,
    AsPath,
};
use crate::assertable::dir_comparison::assert_primary_directory_fully_matches_secondary_directory;


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

    /// Asserts the path points to a direcory, or a symlink to one.
    fn assert_is_directory(&self);

    /// Asserts the path points to a direcory;
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
    /// Structure and exact file contents are compared (two-way),
    /// but **symlinks and file and directory metadata is ignored in the comparison**.
    fn assert_is_directory_and_fully_matches_secondary_directory<P>(&self, other_directory_path: P)
    where
        P: AsPath;

    /// Asserts contents of directory at `other_directory_path` are present in the one at `self`.
    ///
    /// Structure and exact file contents are compared (one-way),
    /// but **symlinks and file and directory metadata is ignored in the comparison**.
    fn assert_is_directory_and_has_contents_of_secondary_directory<P>(
        &self,
        other_directory_path: P,
    ) where
        P: AsPath;


    /*
     * File-related assertions.
     */

    fn assert_is_file(&self);

    fn assert_is_file_and_not_symlink(&self);




    /*
     * Symlink-related assertions.
     */

    fn assert_is_any_symlink(&self);

    fn assert_is_symlink_to_directory(&self);

    fn assert_is_symlink_to_directory_and_destination_matches<P>(&self, expected_destination_path: P) where P: AsRef<Path>;

    fn assert_is_symlink_to_directory_and_resolve_destination(&self) -> PathBuf;

    fn assert_is_symlink_to_file(&self);

    fn assert_is_symlink_to_file_and_destination_matches<P>(&self, expected_destination_path: P) where P: AsRef<Path>;

    fn assert_is_symlink_to_file_and_resolve_destination(&self) -> PathBuf;
}


pub trait ManageablePath {
    fn symlink_to_file<P>(&self, destination_file_path: P)
    where
        P: AsRef<Path>;

    fn symlink_to_directory<P>(&self, destination_directory_path: P)
    where
        P: AsRef<Path>;

    fn touch_if_missing(&self);

    fn file_size_in_bytes(&self) -> u64;

    fn assert_is_file_and_remove(&self);
}

pub trait CaptureableFilePath: AsPath {
    fn capture_with_content(&self) -> CapturedFileState {
        CapturedFileState::new_with_content_capture(self.as_path())
    }
}



fn obtain_path_type<P>(path: &P) -> PathType
where
    P: AsPath,
{
    let metadata_no_follow = path
        .as_path()
        .symlink_metadata()
        .expect("unable to read file metadata");

    let metadata_with_follow = path
        .as_path()
        .metadata()
        .expect("unable to read file metadata (with follow)");

    PathType::from_path_types(
        metadata_no_follow.file_type(),
        metadata_with_follow.file_type(),
    )
}


/// Blanket implementation of [`AssertablePath`] for all items
/// that implement [`AsPath`].
impl<A> AssertablePath for A
where
    A: AsPath,
{
    #[track_caller]
    fn assert_exists(&self) {
        match self.as_path().try_exists() {
            Ok(exists) => assert!(
                exists,
                "path does not exist: {}",
                self.as_path().display()
            ),
            Err(error) => panic!(
                "failed to determine whether the path exists or not (IO error): {}",
                error
            ),
        }
    }

    #[track_caller]
    fn assert_not_exists(&self) {
        match self.as_path().try_exists() {
            Ok(exists) => assert!(
                !exists,
                "path exists: {}",
                self.as_path().display()
            ),
            Err(error) => panic!(
                "failed to determine whether path exists or not: {}",
                error
            ),
        }
    }

    #[track_caller]
    fn assert_is_directory(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::BareDirectory && path_type != PathType::SymlinkToDirectory {
            panic!(
                "path does not lead to a directory (or to a symlink to one), \
                but to {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_directory_and_not_symlink(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::BareDirectory {
            panic!(
                "path does not lead to a non-symlink directory, but to {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_directory_and_empty(&self) {
        self.assert_is_directory();

        let directory_scan = fs::read_dir(self.as_path()).expect("failed to read directory");

        assert!(
            directory_scan.count() == 0,
            "path is directory, but is not empty: {}",
            self.as_path().display()
        )
    }

    #[track_caller]
    fn assert_is_directory_and_not_empty(&self) {
        self.assert_is_directory();

        let directory_scan = fs::read_dir(self.as_path()).expect("failed to read directory");

        assert!(
            directory_scan.count() > 0,
            "path is directory, but is also empty: {}",
            self.as_path().display()
        )
    }

    #[track_caller]
    fn assert_is_directory_and_fully_matches_secondary_directory<P>(
        &self,
        secondary_directory_path: P,
    ) where
        P: AsPath + AssertablePath,
    {
        self.assert_is_directory();
        secondary_directory_path.assert_is_directory();

        assert_primary_directory_fully_matches_secondary_directory(
            self.as_path(),
            secondary_directory_path.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );
    }

    #[track_caller]
    fn assert_is_directory_and_has_contents_of_secondary_directory<P>(
        &self,
        secondary_directory_path: P,
    ) where
        P: AsPath + AssertablePath,
    {
        self.assert_is_directory();
        secondary_directory_path.assert_is_directory();

        assert_primary_directory_precisely_contains_secondary_directory(
            self.as_path(),
            secondary_directory_path.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison: true,
            },
        );
    }


    #[track_caller]
    fn assert_is_file(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::BareFile && path_type != PathType::SymlinkToFile {
            panic!(
                "path does not lead to a file (or a symlink to one), but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_file_and_not_symlink(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::BareFile {
            panic!(
                "path does not lead to a non-symlink file, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_any_symlink(&self) {
        self.assert_exists();

        let metadata_no_follow = self
            .as_path()
            .symlink_metadata()
            .expect("unable to read file metadata without following");

        if !metadata_no_follow.is_symlink() {
            panic!(
                "path is not a symlink, but {:?}: {}",
                metadata_no_follow.file_type(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_symlink_to_directory(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::SymlinkToDirectory {
            panic!(
                "path is not a symlink to a directory, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_symlink_to_directory_and_destination_matches<P>(&self, expected_destination_path: P) where P: AsRef<Path> {
        let canonical_expected_path = expected_destination_path.as_ref().canonicalize()
            .expect("failed to canonicalize expected destination path");

        let destination = self.assert_is_symlink_to_directory_and_resolve_destination();
        let canonical_actual_destination_path = destination.canonicalize().expect("failed to canonicalize symlink destination path");

        assert_eq!(
            canonical_expected_path,
            canonical_actual_destination_path,
            "\"{}\" does not lead to \"{}\", but to \"{}\"",
            self.as_path().display(),
            expected_destination_path.as_ref().display(),
            canonical_actual_destination_path.display()
        );
    }

    #[track_caller]
    fn assert_is_symlink_to_directory_and_resolve_destination(&self) -> PathBuf {
        self.assert_is_symlink_to_directory();

        fs::read_link(self.as_path()).expect("failed to read directory symlink")
    }

    #[track_caller]
    fn assert_is_symlink_to_file(&self) {
        self.assert_exists();

        let path_type = obtain_path_type(self);

        if path_type != PathType::SymlinkToFile {
            panic!(
                "path is not a symlink to a file, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    fn assert_is_symlink_to_file_and_destination_matches<P>(&self, expected_destination_path: P) where P: AsRef<Path> {
        let canonical_expected_path = expected_destination_path.as_ref().canonicalize()
            .expect("failed to canonicalize expected destination path");

        let destination = self.assert_is_symlink_to_file_and_resolve_destination();
        let canonical_actual_destination_path = destination.canonicalize().expect("failed to canonicalize symlink destination path");

        assert_eq!(
            canonical_expected_path,
            canonical_actual_destination_path,
            "\"{}\" does not lead to \"{}\", but to \"{}\"",
            self.as_path().display(),
            expected_destination_path.as_ref().display(),
            canonical_actual_destination_path.display()
        );
    }

    #[track_caller]
    fn assert_is_symlink_to_file_and_resolve_destination(&self) -> PathBuf {
        self.assert_is_symlink_to_file();

        fs::read_link(self.as_path()).expect("failed to read file symlink")
    }
}



/// Creates a symbolic link to a directory.
///
/// `source_path` should point to a non-existent path where the symlink will be created.
/// `target_path` should point to an existing directory to which the symlink will point.
#[track_caller]
fn symlink_to_directory(source_path: &Path, target_path: &Path) {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target_path, source_path)
            .expect("failed to create directory symlink");
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target_path, source_path)
            .expect("failed to create directory symlink");
    }

    #[cfg(not(any(windows, unix)))]
    {
        compile_error!(
            "fs-more supports only the following values of target_family: unix and windows \
                (notably, wasm is unsupported)."
        );
    }
}


/// Creates a symbolic link to a file.
///
/// `source_path` should point to a non-existent path where the symlink will be created.
/// `target_path` should point to an existing file to which the symlink will point.
#[track_caller]
fn symlink_to_file(source_path: &Path, target_path: &Path) {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target_path, source_path)
            .expect("failed to create file symlink");
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target_path, source_path)
            .expect("failed to create file symlink");
    }

    #[cfg(not(any(windows, unix)))]
    {
        compile_error!(
            "fs-more supports only the following values of target_family: unix and windows \
                (notably, wasm is unsupported)."
        );
    }
}



/// Blanket implementation of [`ManageablePath`] for all items
/// that implement [`AsPath`].
impl<A> ManageablePath for A
where
    A: AsPath,
{
    #[track_caller]
    fn symlink_to_file<P>(&self, destination_file_path: P)
    where
        P: AsRef<Path>,
    {
        self.assert_not_exists();

        symlink_to_file(self.as_path(), destination_file_path.as_ref());
    }

    #[track_caller]
    fn symlink_to_directory<P>(&self, destination_directory_path: P)
    where
        P: AsRef<Path>,
    {
        self.assert_not_exists();

        symlink_to_directory(
            self.as_path(),
            destination_directory_path.as_ref(),
        );
    }

    #[track_caller]
    fn touch_if_missing(&self) {
        if !self.as_path().exists() {
            let parent_directory = self
                .as_path()
                .parent()
                .expect("path does not have a parent directory");

            if !parent_directory.exists() {
                fs::create_dir_all(parent_directory)
                    .expect("failed to create missing parent directory");
            }

            fs::File::create_new(self.as_path()).expect("failed to create empty file");
        }
    }

    #[track_caller]
    fn file_size_in_bytes(&self) -> u64 {
        let file_metadata = self
            .as_path()
            .metadata()
            .expect("failed to read file metadata");

        file_metadata.len()
    }

    #[track_caller]
    fn assert_is_file_and_remove(&self) {
        self.assert_is_file();

        fs::remove_file(self.as_path()).expect("failed to remove file");
    }
}
