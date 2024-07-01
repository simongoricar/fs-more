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
//! DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the 
//! test-harness-schema crate).
    
#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]


use std::fs;
use std::path::{PathBuf, Path};
use tempfile::TempDir;
use crate::trees::framework::FileSystemHarness;
use crate::trees::framework::AsInitialFileStateRef;
use crate::trees::framework::AssertableInitialFileCapture;
use crate::trees::framework::FileSystemHarnessDirectory;
use crate::trees::framework::AsRelativePath;
use crate::trees::framework::initialize_empty_file;
use crate::trees::framework::initialize_file_with_string;
use crate::trees::framework::initialize_file_with_random_data;
use crate::assertable::AsPath;
use crate::assertable::AssertablePath;
use crate::assertable::CaptureableFilePath;
use crate::assertable::file::CapturedFileState;
use crate::assertable::file::FileState;
use fs_more_test_harness_generator::schema::FileDataConfiguration;
/**A fs-more filesystem testing harness. Upon calling [`Self::initialize`],
it sets up a temporary directory and initializes the entire configured file tree.
When it's dropped or when [`Self::destroy`] is called, the temporary directory is removed.

In addition to initializing the configured files and directories, a snapshot ("capture")
is created for each file. This is the same as [`CaptureableFilePath::capture_with_content`],but the snapshot is created as tree initialization

This harness has the following entries at the top level:



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
impl FileSystemHarnessDirectory for EmptyTree {}
impl AsRelativePath for EmptyTree {
    fn as_path_relative_to_harness_root(&self) -> &Path {
        Path::new(".")
    }
}
