//! @generated
//! 
//! This code was automatically generated from "simple.json",
//! a file that describes this filesystem tree harness for testing.
//!
//!
//! The full file tree is as follows:
//! ```md
//! .
//! |-- empty.txt (empty)
//! |-- foo
//! |   |-- bar.bin (random data, 16 KiB)
//! |   |-- hello-world.txt (text data, 12 B)
//! ```
//!
//! DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the 
//! test-harness-schema crate).
    
#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]


use std::fs;
use std::path::{PathBuf, Path};
use tempfile::TempDir;
use crate::tree_framework::FileSystemHarness;
use crate::tree_framework::AsInitialFileStateRef;
use crate::tree_framework::AssertableInitialFileCapture;
use crate::tree_framework::FileSystemHarnessDirectory;
use crate::tree_framework::AsRelativePath;
use crate::tree_framework::initialize_empty_file;
use crate::tree_framework::initialize_file_with_string;
use crate::tree_framework::initialize_file_with_random_data;
use crate::assertable::AsPath;
use crate::assertable::r#trait::AssertablePath;
use crate::assertable::r#trait::CaptureableFilePath;
use crate::assertable::file_capture::CapturedFileState;
use crate::assertable::file_capture::FileState;
use fs_more_test_harness_schema::schema::FileDataConfiguration;
/**This is a file residing at `./empty.txt` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct EmptyTxt {
    path: PathBuf,
    initial_state: FileState,
}
impl EmptyTxt {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_empty_file(&path);
        let initial_state = FileState::Empty;
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for EmptyTxt {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for EmptyTxt {}
impl AsInitialFileStateRef for EmptyTxt {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for EmptyTxt {}
impl AsRelativePath for EmptyTxt {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\empty.txt")
    }
}
/**This is a file residing at `./foo/hello-world.txt` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct HelloWorldTxt {
    path: PathBuf,
    initial_state: FileState,
}
impl HelloWorldTxt {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        initialize_file_with_string(&path, "Hello world!");
        let initial_state = FileState::NonEmpty {
            content: Vec::from("Hello world!".as_bytes()),
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for HelloWorldTxt {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for HelloWorldTxt {}
impl AsInitialFileStateRef for HelloWorldTxt {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for HelloWorldTxt {}
impl AsRelativePath for HelloWorldTxt {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\hello-world.txt")
    }
}
/**This is a file residing at `./foo/bar.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct BarBin {
    path: PathBuf,
    initial_state: FileState,
}
impl BarBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &path,
            39581913123u64,
            16384usize,
        );
        let initial_state = FileState::NonEmpty {
            content: binary_file_data,
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for BarBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for BarBin {}
impl AsInitialFileStateRef for BarBin {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for BarBin {}
impl AsRelativePath for BarBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar.bin")
    }
}
/**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

This directory has the following entries:
- `hello_world_txt` (see [`HelloWorldTxt`])
- `bar_bin` (see [`BarBin`])

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct Foo {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/hello-world.txt` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
    pub hello_world_txt: HelloWorldTxt,
    /**This is a file residing at `./foo/bar.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for Foo {}
impl AsRelativePath for Foo {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo")
    }
}
/**A fs-more filesystem testing harness. Upon calling [`Self::initialize`],
it sets up a temporary directory and initializes the entire configured file tree.
When it's dropped or when [`Self::destroy`] is called, the temporary directory is removed.

In addition to initializing the configured files and directories, a snapshot ("capture")
is created for each file. This is the same as [`CaptureableFilePath::capture_with_content`],but the snapshot is created as tree initialization

This harness has the following entries at the top level:
- `empty_txt` (see [`EmptyTxt`])
- `foo` (see [`Foo`])


The full file tree is as follows:
```md
.
|-- empty.txt (empty)
|-- foo
|   |-- bar.bin (random data, 16 KiB)
|   |-- hello-world.txt (text data, 12 B)
```


<br>

<sup>This tree and related code was automatically generated from the structure described in `simple.json`.</sup>*/
pub struct SimpleTree {
    temporary_directory: TempDir,
    /**This is a file residing at `./empty.txt` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
    pub empty_txt: EmptyTxt,
    /**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

This directory has the following entries:
- `hello_world_txt` (see [`HelloWorldTxt`])
- `bar_bin` (see [`BarBin`])

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for SimpleTree {}
impl AsRelativePath for SimpleTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
