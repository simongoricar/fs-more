//! @generated
//! 
//! This code was automatically generated from "simple.json",
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
/**This is a file residing at `./empty.txt` (relative to the root of the test harness).

Part of the [`SimpleTree`] test harness tree.*/
pub struct EmptyTxt {
    path: PathBuf,
}
impl EmptyTxt {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_empty_file(&path);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for EmptyTxt {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for EmptyTxt {}
/**This is a file residing at `./foo/hello-world.txt` (relative to the root of the test harness).

Part of the [`SimpleTree`] test harness tree.*/
pub struct HelloWorldTxt {
    path: PathBuf,
}
impl HelloWorldTxt {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_string(&path, "Hello world!");
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for HelloWorldTxt {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for HelloWorldTxt {}
/**This is a file residing at `./foo/bar.bin` (relative to the root of the test harness).

Part of the [`SimpleTree`] test harness tree.*/
pub struct BarBin {
    path: PathBuf,
}
impl BarBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_random_data(&path, 39581913123u64, 16384usize);
        path.assert_is_file();
        Self { path }
    }
}
impl AsPath for BarBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for BarBin {}
/**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

Part of the [`SimpleTree`] test harness tree.*/
pub struct Foo {
    directory_path: PathBuf,
    pub hello_world_txt: HelloWorldTxt,
    pub bar_bin: BarBin,
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
        let hello_world_txt = <HelloWorldTxt>::new(
            directory_path.clone(),
            "hello-world.txt",
        );
        let bar_bin = <BarBin>::new(directory_path.clone(), "bar.bin");
        Self {
            directory_path,
            hello_world_txt,
            bar_bin,
        }
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

This tree and related code was automatically generated from the structure described in `simple.json`.*/
pub struct SimpleTree {
    temporary_directory: TempDir,
    pub empty_txt: EmptyTxt,
    pub foo: Foo,
}
impl FileSystemHarness for SimpleTree {
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        let empty_txt = <EmptyTxt>::new(
            temporary_directory_path.to_owned(),
            "empty.txt",
        );
        let foo = <Foo>::new(temporary_directory_path.to_owned(), "foo");
        Self {
            temporary_directory,
            empty_txt,
            foo,
        }
    }
    fn destroy(self) {
        self.temporary_directory
            .close()
            .expect("failed to destroy filesystem harness directory");
    }
}
impl AsPath for SimpleTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
