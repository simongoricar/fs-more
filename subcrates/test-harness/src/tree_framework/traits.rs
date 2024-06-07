use std::{
    fs::{self, OpenOptions},
    io::prelude::Read,
    path::{Path, PathBuf},
};

use crate::assertable::{file_capture::FileState, AsPath};

pub trait FileSystemHarness: AsPath {
    /// Initializes the entire filesystem tree harness.
    /// This means setting up a temporary directory and
    /// potentially initializing any directories and files inside,
    /// depending on the given tree definition.
    fn initialize() -> Self;

    /// Consume `self` and remove the entire testing temporary directory.
    fn destroy(self);

    /// Obtain a custom sub-path, by providing a relative `sub_path`.
    fn child_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.as_path().join(sub_path)
    }
}

pub trait FileSystemHarnessSubDirectory: AsPath {
    /*
    fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
    where
        S: Into<String>; */

    fn child_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.as_path().join(sub_path)
    }
}


pub trait AsInitialFileStateRef: AsPath {
    /// For internal use, avoid using this directly in tests.
    fn initial_state(&self) -> &FileState;
}


pub trait AssertableInitialFileCapture: AsInitialFileStateRef {
    /// Assert that the *initial* [`FileState`] (captured for each file at test harness initialization)
    /// matches the *current* state of the file `other`.
    fn assert_initial_state_matches_other_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let captured_other_file = FileState::capture_from_file_path(&other);

        assert!(
            self.initial_state()
                .equals_other_file_state(&captured_other_file),
            "files \"{}\" and \"{}\" don't have equal states",
            self.as_path().display(),
            other.as_ref().display(),
        );
    }

    /// Assert that the *initial* [`FileState`] (captured for each file at test harness initialization)
    /// matches the current state of the same file on disk.
    fn assert_unchanged_from_initial_state(&self) {
        let file_now_exists = self
            .as_path()
            .try_exists()
            .expect("failed to read file metadata");


        match &self.initial_state() {
            FileState::NonExistent => {
                if file_now_exists {
                    panic!(
                        "previous state is NonExistent, but file \"{}\" exists",
                        self.as_path().display()
                    );
                }
            }
            FileState::Empty => {
                if !file_now_exists {
                    panic!(
                        "previous state is Empty, but file \"{}\" does not exist",
                        self.as_path().display()
                    );
                }

                let file = OpenOptions::new()
                    .read(true)
                    .open(self.as_path())
                    .expect("failed to open file");

                if file.bytes().next().is_some() {
                    panic!(
                        "previous state is Empty, but file \"{}\" is not empty",
                        self.as_path().display()
                    );
                }
            }
            FileState::NonEmpty { content } => {
                if !file_now_exists {
                    panic!(
                        "previous state is NonEmpty, but file \"{}\" does not exist",
                        self.as_path().display()
                    );
                }

                let fresh_file_contents =
                    fs::read(self.as_path()).expect("failed to read file contents");

                assert_eq!(
                    content,
                    &fresh_file_contents,
                    "previous state is NonEmpty, but file \"{}\" does not match the captured content",
                    self.as_path().display()
                );
            }
        }
    }
}
