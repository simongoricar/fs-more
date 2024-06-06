use std::path::{Path, PathBuf};

use crate::assertable::AsPath;

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

/*
pub(crate) trait FileSystemHarnessFile: AsPath {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>;
}
 */
