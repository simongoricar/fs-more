use std::{
    fs::{self, OpenOptions},
    io::prelude::Read,
    path::{Path, PathBuf},
};

use crate::assertable::{file::FileState, AsPath};


/// A trait defining the root of the testing harness.
pub trait FileSystemHarness: AsPath {
    /// Initializes the entire filesystem tree harness.
    ///
    /// This means setting up a temporary directory and
    /// initializing any directories and files inside it.
    ///
    /// The details generally depend on the given tree definition,
    /// see the [harness generator] for more info.
    ///
    ///
    /// [harness generator]: fs_more_test_harness_tree_generator
    fn initialize() -> Self;

    /// Consumes `self` and cleans up the entire temporary directory
    /// that was initialized for the harness.
    fn destroy(self);
}



/// A trait defining a sub-directory inside a testing harness.
pub trait FileSystemHarnessDirectory: AsPath {
    /// Obtain a custom sub-path, by providing a relative `sub_path`.
    fn child_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.as_path().join(sub_path)
    }
}


/// An [`AsPath`]-adjacent trait generally implemented for individual entries
/// inside a testing harness, allowing us to obtain their path relative to the
/// base directory of the testing harness.
pub trait AsRelativePath {
    /// Returns a relative path for this file or directory.
    /// The path will be relative to the root of the test harness.
    fn as_path_relative_to_harness_root(&self) -> &Path;
}


/// A trait generally implemented for individual file entries
/// inside a testing harness, allowing us to obtain a reference
/// to their initial [`FileState`].
///
/// See also: [`AssertableInitialFileCapture`].
///
/// *Usage of this trait should not appear directly in tests.*
pub trait AsInitialFileStateRef: AsPath {
    /// For internal use, avoid using this directly in tests.
    fn initial_state(&self) -> &FileState;
}



/// A trait generally implemented for individual file entries
/// inside a testing harness, allowing us to perform comparisons
/// of the current state of the file (or other files) with the
/// initial state of that file as captured on test harness initialization.
///
/// See also: [`AsInitialFileStateRef`].
pub trait AssertableInitialFileCapture: AsInitialFileStateRef {
    /// Asserts that the *initial* [`FileState`] (captured for each file at test harness initialization)
    /// matches the *current* state of the file `other`.
    #[track_caller]
    fn assert_initial_state_matches_other_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let captured_other_file = FileState::capture_from_file_path(&other);

        assert!(
            self.initial_state()
                .equals_other_file_state(&captured_other_file),
            "initial capture of \"{}\" and \"{}\" don't have equal states: \n{:?} vs {:?}",
            self.as_path().display(),
            other.as_ref().display(),
            self.initial_state(),
            captured_other_file
        );
    }

    /// Asserts that the *initial* [`FileState`] (captured for each file at test harness initialization)
    /// matches the current state of the same file on disk.
    #[track_caller]
    fn assert_unchanged_from_initial_state(&self) {
        let file_now_exists = self
            .as_path()
            .try_exists()
            .expect("failed to read file metadata");


        match &self.initial_state() {
            FileState::NonExistent => {
                if file_now_exists {
                    panic!(
                        "initial state is NonExistent, but file \"{}\" exists",
                        self.as_path().display()
                    );
                }
            }
            FileState::Empty => {
                if !file_now_exists {
                    panic!(
                        "initial state is Empty, but file \"{}\" does not exist",
                        self.as_path().display()
                    );
                }

                let file = OpenOptions::new()
                    .read(true)
                    .open(self.as_path())
                    .expect("failed to open file");

                if file.bytes().next().is_some() {
                    panic!(
                        "initial state is Empty, but file \"{}\" is not empty",
                        self.as_path().display()
                    );
                }
            }
            FileState::NonEmpty { content } => {
                if !file_now_exists {
                    panic!(
                        "initial state is NonEmpty, but file \"{}\" does not exist",
                        self.as_path().display()
                    );
                }

                let fresh_file_contents =
                    fs::read(self.as_path()).expect("failed to read file contents");

                assert_eq!(
                    content,
                    &fresh_file_contents,
                    "initial state is NonEmpty, but file \"{}\" does not match the captured content",
                    self.as_path().display()
                );
            }
        }
    }
}
