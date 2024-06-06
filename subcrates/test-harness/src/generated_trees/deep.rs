//! @generated
//! 
//! This code was automatically generated from "deep.json",
//! describing a filesystem tree harness for testing.
//!
//! DO NOT MODIFY THIS FILE, MODIFY THE JSON DATA FILE AND
//! REGENERATE THIS FILE INSTEAD (see test-harness-schema crate).
    
#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]


use std::fs;
use std::path::{PathBuf, Path};
use tempfile::TempDir;
use crate::tree_framework::FileSystemHarness;
use crate::tree_framework::initialize_empty_file;
use crate::tree_framework::initialize_file_with_string;
use crate::tree_framework::initialize_file_with_random_data;
use crate::assertable::AsPath;
use crate::assertable::r#trait::AssertablePath;
use crate::assertable::r#trait::CaptureableFilePath;
use crate::assertable::file_capture::CapturedFileState;
use fs_more_test_harness_schema::schema::FileDataConfiguration;
/**This is a file residing at `./a.bin` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct ABin {
    path: PathBuf,
}
impl ABin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_random_data(&path, 12345u64, 32768usize);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for ABin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for ABin {}
/**This is a file residing at `./foo/b.bin` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct BBin {
    path: PathBuf,
}
impl BBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_random_data(&path, 54321u64, 65536usize);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for BBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for BBin {}
/**This is a file residing at `./foo/bar/c.bin` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct CBin {
    path: PathBuf,
}
impl CBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_random_data(&path, 54321u64, 131072usize);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for CBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for CBin {}
/**This is a file residing at `./foo/bar/hello/world/d.bin` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct DBin {
    path: PathBuf,
}
impl DBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_random_data(&path, 54321u64, 262144usize);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for DBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for DBin {}
/**This is a sub-directory residing at `./foo/bar/hello/world` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct World {
    directory_path: PathBuf,
    pub d_bin: DBin,
}
impl World {
    fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
    where
        S: Into<String>,
    {
        let directory_path = parent_path.join(directory_name.into());
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let d_bin = <DBin>::new(directory_path.clone(), "d.bin");
        Self { directory_path, d_bin }
    }
}
impl AsPath for World {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
/**This is a sub-directory residing at `./foo/bar/hello` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct Hello {
    directory_path: PathBuf,
    pub world: World,
}
impl Hello {
    fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
    where
        S: Into<String>,
    {
        let directory_path = parent_path.join(directory_name.into());
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let world = <World>::new(directory_path.clone(), "world");
        Self { directory_path, world }
    }
}
impl AsPath for Hello {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
/**This is a sub-directory residing at `./foo/bar` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct Bar {
    directory_path: PathBuf,
    pub c_bin: CBin,
    pub hello: Hello,
}
impl Bar {
    fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
    where
        S: Into<String>,
    {
        let directory_path = parent_path.join(directory_name.into());
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let c_bin = <CBin>::new(directory_path.clone(), "c.bin");
        let hello = <Hello>::new(directory_path.clone(), "hello");
        Self {
            directory_path,
            c_bin,
            hello,
        }
    }
}
impl AsPath for Bar {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
/**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

Part of the [`DeepTree`] test harness tree.*/
pub struct Foo {
    directory_path: PathBuf,
    pub b_bin: BBin,
    pub bar: Bar,
}
impl Foo {
    fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
    where
        S: Into<String>,
    {
        let directory_path = parent_path.join(directory_name.into());
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let b_bin = <BBin>::new(directory_path.clone(), "b.bin");
        let bar = <Bar>::new(directory_path.clone(), "bar");
        Self { directory_path, b_bin, bar }
    }
}
impl AsPath for Foo {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
/**A fs-more filesystem testing harness. Upon calling [`Self::initialize`],
it sets up a temporary directory and initializes the entire configured file tree.
When it's dropped or when [`Self::destroy`] is called, the temporary directory is removed.

This tree and related code was automatically generated from the structure described in `deep.json`.*/
pub struct DeepTree {
    temporary_directory: TempDir,
    pub a_bin: ABin,
    pub foo: Foo,
}
impl FileSystemHarness for DeepTree {
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        let a_bin = <ABin>::new(temporary_directory_path.to_owned(), "a.bin");
        let foo = <Foo>::new(temporary_directory_path.to_owned(), "foo");
        Self {
            temporary_directory,
            a_bin,
            foo,
        }
    }
    fn destroy(self) {
        self.temporary_directory
            .close()
            .expect("failed to destroy filesystem harness directory");
    }
}
impl AsPath for DeepTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
