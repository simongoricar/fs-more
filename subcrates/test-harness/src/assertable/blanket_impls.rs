use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::{
    directory::{
        assert_primary_directory_fully_matches_secondary_directory,
        assert_primary_directory_precisely_contains_secondary_directory,
        DirectoryComparisonOptions,
    },
    path_type::PathType,
    AsPath,
    AssertablePath,
    ManageablePath,
    WithSubPath,
};


/// Blanket implements `AsPath` for all `AsRef<Path>`s.
impl<P> AsPath for P
where
    P: AsRef<Path>,
{
    fn as_path(&self) -> &Path {
        self.as_ref()
    }
}


/// Blanket implementss `WithSubPath` for all `AsPath`s.
impl<A> WithSubPath for A
where
    A: AsPath,
{
    fn sub_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.as_path().join(sub_path)
    }
}



/// Blanket implementation of [`AssertablePath`] for all items
/// that implement [`AsPath`].
impl<A> AssertablePath for A
where
    A: AsPath,
{
    #[track_caller]
    fn assert_exists(&self) {
        match fs::symlink_metadata(self.as_path()) {
            Ok(_) => {},
            Err(error) => match error.kind() {
                ErrorKind::NotFound => panic!("path does not exist: {}", self.as_path().display()),
                _ => panic!(
                    "failed to determine whether the path exists or not (IO error): {} (for path {})",
                    error,
                    self.as_path().display()
                ),
            },
        };
    }

    #[track_caller]
    fn assert_not_exists(&self) {
        match fs::symlink_metadata(self.as_path()) {
            Ok(_) => panic!("path exists: {}", self.as_path().display()),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => {},
                _ => panic!(
                    "failed to determine whether the path exists or not (IO error): {} (for path {})",
                    error,
                    self.as_path().display()
                ),
            },
        };
    }

    #[track_caller]
    fn assert_is_directory(&self) {
        self.assert_exists();

        let path_type = PathType::from_path(self.as_path()).unwrap();

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

        let path_type = PathType::from_path(self.as_path()).unwrap();

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
    fn assert_is_directory_and_fully_matches_secondary_directory_with_options<P>(
        &self,
        secondary_directory_path: P,
        strict_symlink_comparison: bool,
    ) where
        P: AsPath + AssertablePath,
    {
        self.assert_is_directory();
        secondary_directory_path.assert_is_directory();

        assert_primary_directory_fully_matches_secondary_directory(
            self.as_path(),
            secondary_directory_path.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison,
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
    fn assert_is_directory_and_has_contents_of_secondary_directory_with_options<P>(
        &self,
        secondary_directory_path: P,
        strict_symlink_comparison: bool,
    ) where
        P: AsPath + AssertablePath,
    {
        self.assert_is_directory();
        secondary_directory_path.assert_is_directory();

        assert_primary_directory_precisely_contains_secondary_directory(
            self.as_path(),
            secondary_directory_path.as_path(),
            DirectoryComparisonOptions {
                strict_symlink_comparison,
            },
        );
    }


    #[track_caller]
    fn assert_is_file(&self) {
        self.assert_exists();

        let path_type = PathType::from_path(self.as_path()).unwrap();

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

        let path_type = PathType::from_path(self.as_path()).unwrap();

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
    fn assert_is_any_valid_symlink(&self) {
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


        let symlink_destination = self
            .as_path()
            .read_link()
            .expect("unable to read file symlink destination");

        symlink_destination.assert_exists();
    }

    #[track_caller]
    fn assert_is_any_broken_symlink(&self) {
        self.assert_exists();

        let metadata_no_follow = self
            .as_path()
            .symlink_metadata()
            .expect("unable to read file metadata without following");

        if !metadata_no_follow.is_symlink() {
            let path_type = PathType::from_path(self.as_path()).unwrap();

            panic!(
                "path is not a symlink, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }


        let symlink_destination = self
            .as_path()
            .read_link()
            .expect("unable to read file symlink destination");

        symlink_destination.assert_not_exists();
    }

    #[track_caller]
    fn assert_is_valid_symlink_to_directory(&self) {
        self.assert_exists();

        let path_type = PathType::from_path(self.as_path()).unwrap();

        if path_type != PathType::SymlinkToDirectory {
            panic!(
                "path is not a symlink to a directory, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    #[track_caller]
    fn assert_is_valid_symlink_to_directory_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>,
    {
        self.assert_exists();


        let canonical_expected_path = expected_destination_path
            .as_ref()
            .canonicalize()
            .expect("failed to canonicalize expected destination path");

        let destination = self.assert_is_valid_symlink_to_directory_and_resolve_destination();
        let canonical_actual_destination_path = destination
            .canonicalize()
            .expect("failed to canonicalize symlink destination path");

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
    fn assert_is_broken_symlink_to_directory_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>,
    {
        self.assert_is_any_broken_symlink();


        let resolved_symlink_destination =
            fs::read_link(self.as_path()).expect("failed to read directory symlink");

        resolved_symlink_destination.assert_not_exists();

        assert_eq!(
            expected_destination_path.as_ref(),
            resolved_symlink_destination,
            "\"{}\" should be a broken symbolic link to \"{}\", \
            but it points to \"{}\" instead",
            self.as_path().display(),
            expected_destination_path.as_ref().display(),
            resolved_symlink_destination.display()
        );
    }

    #[track_caller]
    fn assert_is_valid_symlink_to_directory_and_resolve_destination(&self) -> PathBuf {
        self.assert_is_valid_symlink_to_directory();

        fs::read_link(self.as_path()).expect("failed to read directory symlink")
    }

    #[track_caller]
    fn assert_is_valid_symlink_to_file(&self) {
        self.assert_exists();

        let path_type = PathType::from_path(self.as_path()).unwrap();

        if path_type != PathType::SymlinkToFile {
            panic!(
                "path is not a symlink to a file, but {}: {}",
                path_type.to_short_name(),
                self.as_path().display()
            );
        }
    }

    fn assert_is_valid_symlink_to_file_and_destination_matches<P>(
        &self,
        expected_destination_path: P,
    ) where
        P: AsRef<Path>,
    {
        let canonical_expected_path = expected_destination_path
            .as_ref()
            .canonicalize()
            .expect("failed to canonicalize expected destination path");

        let destination = self.assert_is_valid_symlink_to_file_and_resolve_destination();
        let canonical_actual_destination_path = destination
            .canonicalize()
            .expect("failed to canonicalize symlink destination path");

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
    fn assert_is_valid_symlink_to_file_and_resolve_destination(&self) -> PathBuf {
        self.assert_is_valid_symlink_to_file();

        fs::read_link(self.as_path()).expect("failed to read file symlink")
    }
}



/// Creates a symbolic link to a directory.
///
/// `source_path` should point to a non-existent path where the symlink will be created.
/// `target_path` should point to an existing directory to which the symlink will point.
///
/// # Panics
/// This function will panic if the symbolic link cannot be created for any reason.
/// This is acceptable in our case because this code is used for the test harness.
#[track_caller]
pub(crate) fn symlink_to_directory(source_path: &Path, target_path: &Path) {
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
///
/// # Panics
/// This function will panic if the symbolic link cannot be created for any reason.
/// This is acceptable in our case because this code is used for the test harness.
#[track_caller]
pub(crate) fn symlink_to_file(source_path: &Path, target_path: &Path) {
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

        self.assert_is_valid_symlink_to_file_and_destination_matches(destination_file_path);
    }

    #[track_caller]
    fn symlink_to_directory<P>(&self, destination_directory_path: P)
    where
        P: AsRef<Path>,
    {
        self.assert_not_exists();

        symlink_to_directory(self.as_path(), destination_directory_path.as_ref());

        self.assert_is_valid_symlink_to_directory_and_destination_matches(
            destination_directory_path,
        );
    }

    #[track_caller]
    fn assert_not_exists_and_create_empty_directory(&self) {
        self.assert_not_exists();

        fs::create_dir_all(self.as_path()).expect("failed to create empty directory");

        self.assert_is_directory_and_empty();
    }

    #[track_caller]
    fn assert_not_exists_and_create_empty_file(&self) {
        self.assert_not_exists();

        let parent_directory = self
            .as_path()
            .parent()
            .expect("path does not have a parent directory");

        if !parent_directory.exists() {
            fs::create_dir_all(parent_directory)
                .expect("failed to create missing parent directory");
        }

        fs::OpenOptions::new()
            .create_new(true)
            .open(self.as_path())
            .expect("failed to create empty file");

        self.assert_is_file_and_not_symlink();
    }

    #[track_caller]
    fn size_in_bytes(&self) -> u64 {
        self.assert_exists();

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

    #[track_caller]
    fn assert_is_empty_directory_and_remove(&self) {
        self.assert_is_directory_and_empty();

        fs::remove_dir(self.as_path()).expect("failed to remove empty directory");

        self.assert_not_exists();
    }

    #[track_caller]
    fn assert_is_symlink_and_remove(&self) {
        self.assert_is_any_symlink();

        let resolved_destination_path =
            fs::read_link(self.as_path()).expect("failed to follow symlink");

        resolved_destination_path.assert_exists();


        #[cfg(unix)]
        {
            fs::remove_file(self.as_path()).expect("failed to remove symlink");
        }

        #[cfg(windows)]
        {
            fs::remove_dir(self.as_path()).expect("failed to remove symlink");
        }

        #[cfg(not(any(unix, windows)))]
        {
            compile_error!(
                "fs-more's test harness supports only the following values of target_family: \
                unix and windows (notably, wasm is unsupported)."
            );
        }

        resolved_destination_path.assert_exists();

        self.assert_not_exists();
    }
}
