//! @generated
//!
//! This code was automatically generated from "deep.json",
//! a file that describes this filesystem tree harness for testing.
//!
//!
//! The full file tree is as follows:
//! ```md
//! .
//! |-> a.bin (binary data, 32 KiB)
//! |-- foo
//! |   |-- bar
//! |   |   |-- hello
//! |   |   |   |-- world
//! |   |   |   |   |-> d.bin (binary data, 256 KiB)
//! |   |   |-> c.bin (binary data, 128 KiB)
//! |   |-> b.bin (binary data, 64 KiB)
//! ```
//!
//! <sup>DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the
//! test-harness-schema crate).</sup>

#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]
#![allow(unused)]


use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use crate::prelude::*;
use crate::trees::{
    initialize_empty_file, initialize_file_with_string, initialize_file_with_random_data,
    initialize_symbolic_link, SymlinkDestinationType, AsInitialFileStateRef,
};
use fs_more_test_harness_tree_schema::schema::FileDataConfiguration;
/**This is a file residing at `./a.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct ABin {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl ABin {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("a.bin");
        file_path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &file_path,
            12345u64,
            32768usize,
        );
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::NonEmpty {
            content: binary_file_data,
        };
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for ABin {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for ABin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./a.bin")
    }
}
impl AsInitialFileStateRef for ABin {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for ABin {}
impl CaptureableFilePath for ABin {}
/**This is a file residing at `./foo/b.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct BBin {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl BBin {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("b.bin");
        file_path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &file_path,
            54321u64,
            65536usize,
        );
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::NonEmpty {
            content: binary_file_data,
        };
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for BBin {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for BBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/b.bin")
    }
}
impl AsInitialFileStateRef for BBin {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for BBin {}
impl CaptureableFilePath for BBin {}
/**This is a file residing at `./foo/bar/c.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct CBin {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl CBin {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("c.bin");
        file_path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &file_path,
            54321u64,
            131072usize,
        );
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::NonEmpty {
            content: binary_file_data,
        };
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for CBin {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for CBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/bar/c.bin")
    }
}
impl AsInitialFileStateRef for CBin {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for CBin {}
impl CaptureableFilePath for CBin {}
/**This is a file residing at `./foo/bar/hello/world/d.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct DBin {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl DBin {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("d.bin");
        file_path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &file_path,
            54321u64,
            262144usize,
        );
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::NonEmpty {
            content: binary_file_data,
        };
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for DBin {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for DBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/bar/hello/world/d.bin")
    }
}
impl AsInitialFileStateRef for DBin {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for DBin {}
impl CaptureableFilePath for DBin {}
/**This is a sub-directory residing at `./foo/bar/hello/world` (relative to the root of the test harness).


It contains the following files:
- `d.bin` (field `d_bin`; see [`DBin`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct World {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/bar/hello/world/d.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub d_bin: DBin,
}
impl World {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let directory_path = parent_directory_path.join("world");
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let d_bin = <DBin>::initialize(&directory_path);
        Self { directory_path, d_bin }
    }
}
impl AsPath for World {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
impl AsRelativePath for World {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/bar/hello/world")
    }
}
impl FileSystemHarnessDirectory for World {}
/**This is a sub-directory residing at `./foo/bar/hello` (relative to the root of the test harness).


It contains the following sub-directories:
- `world` (field `world`; see [`World`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Hello {
    directory_path: PathBuf,
    /**This is a sub-directory residing at `./foo/bar/hello/world` (relative to the root of the test harness).


It contains the following files:
- `d.bin` (field `d_bin`; see [`DBin`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub world: World,
}
impl Hello {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let directory_path = parent_directory_path.join("hello");
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let world = <World>::initialize(&directory_path);
        Self { directory_path, world }
    }
}
impl AsPath for Hello {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
impl AsRelativePath for Hello {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/bar/hello")
    }
}
impl FileSystemHarnessDirectory for Hello {}
/**This is a sub-directory residing at `./foo/bar` (relative to the root of the test harness).


It contains the following sub-directories:
- `hello` (field `hello`; see [`Hello`])
It contains the following files:
- `c.bin` (field `c_bin`; see [`CBin`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Bar {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/bar/c.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub c_bin: CBin,
    /**This is a sub-directory residing at `./foo/bar/hello` (relative to the root of the test harness).


It contains the following sub-directories:
- `world` (field `world`; see [`World`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub hello: Hello,
}
impl Bar {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let directory_path = parent_directory_path.join("bar");
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let c_bin = <CBin>::initialize(&directory_path);
        let hello = <Hello>::initialize(&directory_path);
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
impl AsRelativePath for Bar {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo/bar")
    }
}
impl FileSystemHarnessDirectory for Bar {}
/**This is a sub-directory residing at `./foo` (relative to the root of the test harness).


It contains the following sub-directories:
- `bar` (field `bar`; see [`Bar`])
It contains the following files:
- `b.bin` (field `b_bin`; see [`BBin`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
pub struct Foo {
    directory_path: PathBuf,
    /**This is a file residing at `./foo/b.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub b_bin: BBin,
    /**This is a sub-directory residing at `./foo/bar` (relative to the root of the test harness).


It contains the following sub-directories:
- `hello` (field `hello`; see [`Hello`])
It contains the following files:
- `c.bin` (field `c_bin`; see [`CBin`])


<br>

<sup>This entry is part of the [`DeepTree`] test harness tree.</sup>*/
    pub bar: Bar,
}
impl Foo {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let directory_path = parent_directory_path.join("foo");
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let b_bin = <BBin>::initialize(&directory_path);
        let bar = <Bar>::initialize(&directory_path);
        Self { directory_path, b_bin, bar }
    }
}
impl AsPath for Foo {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
impl AsRelativePath for Foo {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./foo")
    }
}
impl FileSystemHarnessDirectory for Foo {}
/**`fs-more` filesystem tree for testing. Upon calling [`DeepTree::initialize`],
a temporary directory is set up, and the entire pre-defined filesystem tree is initialized.
When [`DeepTree::destroy`] is called (or when the struct is dropped), the temporary directory is removed,
along with all of its contents.

In addition to initializing the configured files and directories, a snapshot is created
for each file (also called a "capture"). This is the same as [`CaptureableFilePath::capture_with_content`],but the snapshot is recorded at tree initialization.

This harness has the following sub-entries at the top level (files, sub-directories, ...):
- `a_bin` (see [`ABin`])
- `foo` (see [`Foo`])


The full file tree is as follows:
```md
.
|-> a.bin (binary data, 32 KiB)
|-- foo
|   |-- bar
|   |   |-- hello
|   |   |   |-- world
|   |   |   |   |-> d.bin (binary data, 256 KiB)
|   |   |-> c.bin (binary data, 128 KiB)
|   |-> b.bin (binary data, 64 KiB)
```


<br>

<sup>This tree and related code was automatically generated from the structure described in `deep.json`.</sup>*/
pub struct DeepTree {
    temporary_directory: TempDir,
    pub a_bin: ABin,
    pub foo: Foo,
}
impl FileSystemHarness for DeepTree {
    #[track_caller]
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        let a_bin = <ABin>::initialize(temporary_directory_path);
        let foo = <Foo>::initialize(temporary_directory_path);
        Self {
            temporary_directory,
            a_bin,
            foo,
        }
    }
    #[track_caller]
    fn destroy(self) {
        if self.temporary_directory.path().exists() {
            self.temporary_directory
                .close()
                .expect("failed to destroy filesystem harness directory");
        } else {
            println!(
                "Temporary directory \"{}\" doesn't exist, no need to clean up.", self
                .temporary_directory.path().display()
            );
        }
    }
}
impl AsPath for DeepTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
impl AsRelativePath for DeepTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
impl FileSystemHarnessDirectory for DeepTree {}
