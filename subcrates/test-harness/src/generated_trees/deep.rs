//! @generated
//! 
//! This code was automatically generated from "deep.json",
//! a file that describes this filesystem tree harness for testing.
//!
//!
//! The full file tree is as follows:
//! ```md
//! .
//! |-- a.bin (random data, 32 KiB)
//! |-- foo
//! |   |-- bar
//! |   |   |-- hello
//! |   |   |   |-- world
//! |   |   |   |   |-- d.bin (random data, 256 KiB)
//! |   |   |-- c.bin (random data, 128 KiB)
//! |   |-- b.bin (random data, 64 KiB)
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
/**This is a file residing at `./a.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct ABin {
    path: PathBuf,
    initial_state: FileState,
}
impl ABin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &path,
            12345u64,
            32768usize,
        );
        let initial_state = FileState::NonEmpty {
            content: binary_file_data,
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for ABin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for ABin {}
impl AsInitialFileStateRef for ABin {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for ABin {}
impl AsRelativePath for ABin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\a.bin")
    }
}
/**This is a file residing at `./foo/b.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct BBin {
    path: PathBuf,
    initial_state: FileState,
}
impl BBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &path,
            54321u64,
            65536usize,
        );
        let initial_state = FileState::NonEmpty {
            content: binary_file_data,
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for BBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for BBin {}
impl AsInitialFileStateRef for BBin {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for BBin {}
impl AsRelativePath for BBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\b.bin")
    }
}
/**This is a file residing at `./foo/bar/c.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct CBin {
    path: PathBuf,
    initial_state: FileState,
}
impl CBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &path,
            54321u64,
            131072usize,
        );
        let initial_state = FileState::NonEmpty {
            content: binary_file_data,
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for CBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for CBin {}
impl AsInitialFileStateRef for CBin {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for CBin {}
impl AsRelativePath for CBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar\\c.bin")
    }
}
/**This is a file residing at `./foo/bar/hello/world/d.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct DBin {
    path: PathBuf,
    initial_state: FileState,
}
impl DBin {
    fn new<S>(parent_path: PathBuf, file_name: S) -> Self
    where
        S: Into<String>,
    {
        let path = parent_path.join(file_name.into());
        path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &path,
            54321u64,
            262144usize,
        );
        let initial_state = FileState::NonEmpty {
            content: binary_file_data,
        };
        path.assert_is_file();
        Self { path, initial_state }
    }
}
impl AsPath for DBin {
    fn as_path(&self) -> &Path {
        &self.path
    }
}
impl CaptureableFilePath for DBin {}
impl AsInitialFileStateRef for DBin {
    fn initial_state(&self) -> &FileState {
        &self.initial_state
    }
}
impl AssertableInitialFileCapture for DBin {}
impl AsRelativePath for DBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar\\hello\\world\\d.bin")
    }
}
/**This is a sub-directory residing at `./foo/bar/hello/world` (relative to the root of the test harness).

This directory has the following entries:
- `d_bin` (see [`DBin`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct World {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/bar/hello/world/d.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for World {}
impl AsRelativePath for World {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar\\hello\\world")
    }
}
/**This is a sub-directory residing at `./foo/bar/hello` (relative to the root of the test harness).

This directory has the following entries:
- `world` (see [`World`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Hello {
    directory_path: PathBuf,
    /**This is a sub-directory residing at `./foo/bar/hello/world` (relative to the root of the test harness).

This directory has the following entries:
- `d_bin` (see [`DBin`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for Hello {}
impl AsRelativePath for Hello {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar\\hello")
    }
}
/**This is a sub-directory residing at `./foo/bar` (relative to the root of the test harness).

This directory has the following entries:
- `c_bin` (see [`CBin`])
- `hello` (see [`Hello`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Bar {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/bar/c.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub c_bin: CBin,
    /**This is a sub-directory residing at `./foo/bar/hello` (relative to the root of the test harness).

This directory has the following entries:
- `world` (see [`World`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for Bar {}
impl AsRelativePath for Bar {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".\\foo\\bar")
    }
}
/**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

This directory has the following entries:
- `b_bin` (see [`BBin`])
- `bar` (see [`Bar`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Foo {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/b.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub b_bin: BBin,
    /**This is a sub-directory residing at `./foo/bar` (relative to the root of the test harness).

This directory has the following entries:
- `c_bin` (see [`CBin`])
- `hello` (see [`Hello`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
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
- `a_bin` (see [`ABin`])
- `foo` (see [`Foo`])


The full file tree is as follows:
```md
.
|-- a.bin (random data, 32 KiB)
|-- foo
|   |-- bar
|   |   |-- hello
|   |   |   |-- world
|   |   |   |   |-- d.bin (random data, 256 KiB)
|   |   |-- c.bin (random data, 128 KiB)
|   |-- b.bin (random data, 64 KiB)
```


<br>

<sup>This tree and related code was automatically generated from the structure described in `deep.json`.</sup>*/
pub struct DeepTree {
    temporary_directory: TempDir,
    /**This is a file residing at `./a.bin` (relative to the root of the test harness).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub a_bin: ABin,
    /**This is a sub-directory residing at `./foo` (relative to the root of the test harness).

This directory has the following entries:
- `b_bin` (see [`BBin`])
- `bar` (see [`Bar`])

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
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
impl FileSystemHarnessDirectory for DeepTree {}
impl AsRelativePath for DeepTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
