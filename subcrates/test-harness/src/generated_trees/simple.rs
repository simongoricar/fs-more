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
//! |-- yes
//! |   |-- no.bin (random data, 16 KiB)
//! |   |-- hello-world.txt (text data, 12 B)
//! ```
//!
//! <sup>DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the
//! test-harness-schema crate).</sup>

#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]


use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use crate::prelude::*;
use crate::trees::{
    initialize_empty_file, initialize_file_with_string, initialize_file_with_random_data,
    initialize_symbolic_link, SymlinkDestinationType, AsInitialFileStateRef,
};
use fs_more_test_harness_generator::schema::FileDataConfiguration;
/**This is a file residing at `./empty.txt` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct EmptyTxt {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl EmptyTxt {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("EmptyTxt");
        file_path.assert_not_exists();
        initialize_empty_file(&file_path);
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::Empty;
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for EmptyTxt {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for EmptyTxt {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./empty.txt")
    }
}
impl AsInitialFileStateRef for EmptyTxt {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for EmptyTxt {}
impl CaptureableFilePath for EmptyTxt {}
/**This is a file residing at `./yes/hello-world.txt` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct HelloWorldTxt {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl HelloWorldTxt {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("HelloWorldTxt");
        file_path.assert_not_exists();
        initialize_file_with_string(&file_path, "Hello world!");
        file_path.assert_is_file_and_not_symlink();
        let state_at_initialization = FileState::NonEmpty {
            content: Vec::from("Hello world!".as_bytes()),
        };
        Self {
            file_path,
            state_at_initialization,
        }
    }
}
impl AsPath for HelloWorldTxt {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for HelloWorldTxt {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./yes/hello-world.txt")
    }
}
impl AsInitialFileStateRef for HelloWorldTxt {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for HelloWorldTxt {}
impl CaptureableFilePath for HelloWorldTxt {}
/**This is a file residing at `./yes/no.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct NoBin {
    file_path: PathBuf,
    state_at_initialization: FileState,
}
impl NoBin {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let file_path = parent_directory_path.join("NoBin");
        file_path.assert_not_exists();
        let binary_file_data = initialize_file_with_random_data(
            &file_path,
            39581913123u64,
            16384usize,
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
impl AsPath for NoBin {
    fn as_path(&self) -> &Path {
        &self.file_path
    }
}
impl AsRelativePath for NoBin {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./yes/no.bin")
    }
}
impl AsInitialFileStateRef for NoBin {
    fn initial_state(&self) -> &FileState {
        &self.state_at_initialization
    }
}
impl AssertableInitialFileCapture for NoBin {}
impl CaptureableFilePath for NoBin {}
/**This is a sub-directory residing at `./yes` (relative to the root of the test harness).


It contains the following files:
- `HelloWorldTxt` (field `hello_world_txt`; see [`HelloWorldTxt`])
- `NoBin` (field `no_bin`; see [`NoBin`])


<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
pub struct Yes {
    directory_path: PathBuf,
    /**This is a file residing at `./yes/hello-world.txt` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
    pub hello_world_txt: HelloWorldTxt,
    /**This is a file residing at `./yes/no.bin` (relative to the root of the tree).

<br>

<sup>This entry is part of the [`SimpleTree`] test harness tree.</sup>*/
    pub no_bin: NoBin,
}
impl Yes {
    #[track_caller]
    fn initialize(parent_directory_path: &Path) -> Self {
        let directory_path = parent_directory_path.join("yes");
        directory_path.assert_not_exists();
        fs::create_dir(&directory_path).expect("failed to create directory");
        directory_path.assert_is_directory_and_empty();
        let hello_world_txt = <HelloWorldTxt>::initialize(&directory_path);
        let no_bin = <NoBin>::initialize(&directory_path);
        Self {
            directory_path,
            hello_world_txt,
            no_bin,
        }
    }
}
impl AsPath for Yes {
    fn as_path(&self) -> &Path {
        &self.directory_path
    }
}
impl AsRelativePath for Yes {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new("./yes")
    }
}
impl FileSystemHarnessDirectory for Yes {}
/**`fs-more` filesystem tree for testing. Upon calling [`SimpleTree::initialize`],
a temporary directory is set up, and the entire pre-defined filesystem tree is initialized.
When [`SimpleTree::destroy`] is called (or when the struct is dropped), the temporary directory is removed,
along with all of its contents.

In addition to initializing the configured files and directories, a snapshot is created
for each file (also called a "capture"). This is the same as [`CaptureableFilePath::capture_with_content`],but the snapshot is recorded at tree initialization.

This harness has the following sub-entries at the top level (files, sub-directories, ...):
- `empty_txt` (see [`EmptyTxt`])
- `yes` (see [`Yes`])


The full file tree is as follows:
```md
.
|-- empty.txt (empty)
|-- yes
|   |-- no.bin (random data, 16 KiB)
|   |-- hello-world.txt (text data, 12 B)
```


<br>

<sup>This tree and related code was automatically generated from the structure described in `simple.json`.</sup>*/
pub struct SimpleTree {
    temporary_directory: TempDir,
    empty_txt: EmptyTxt,
    yes: Yes,
}
impl FileSystemHarness for SimpleTree {
    #[track_caller]
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        let empty_txt = <EmptyTxt>::initialize(temporary_directory_path);
        let yes = <Yes>::initialize(temporary_directory_path);
        Self {
            temporary_directory,
            empty_txt,
            yes,
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
impl AsPath for SimpleTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
impl AsRelativePath for SimpleTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
impl FileSystemHarnessDirectory for SimpleTree {}
