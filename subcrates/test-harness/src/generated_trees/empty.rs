//! @generated
//!
//! This code was automatically generated from "empty.json",
//! a file that describes this filesystem tree harness for testing.
//!
//!
//! The full file tree is as follows:
//! ```md
//! .
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
/**`fs-more` filesystem tree for testing. Upon calling [`EmptyTree::initialize`],
a temporary directory is set up, and the entire pre-defined filesystem tree is initialized.
When [`EmptyTree::destroy`] is called (or when the struct is dropped), the temporary directory is removed,
along with all of its contents.

In addition to initializing the configured files and directories, a snapshot is created
for each file (also called a "capture"). This is the same as [`CaptureableFilePath::capture_with_content`],but the snapshot is recorded at tree initialization.

This harness has the following sub-entries at the top level (files, sub-directories, ...):



The full file tree is as follows:
```md
.
```


<br>

<sup>This tree and related code was automatically generated from the structure described in `empty.json`.</sup>*/
pub struct EmptyTree {
    temporary_directory: TempDir,
}
impl FileSystemHarness for EmptyTree {
    #[track_caller]
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        Self { temporary_directory }
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
impl AsPath for EmptyTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
impl AsRelativePath for EmptyTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
impl FileSystemHarnessDirectory for EmptyTree {}
