//! @generated
//! 
//! This code was automatically generated from "empty.json",
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
/**A fs-more filesystem testing harness. Upon calling [`Self::initialize`],
it sets up a temporary directory and initializes the entire configured file tree.
When it's dropped or when [`Self::destroy`] is called, the temporary directory is removed.

This tree and related code was automatically generated from the structure described in `empty.json`.*/
pub struct EmptyTree {
    temporary_directory: TempDir,
}
impl FileSystemHarness for EmptyTree {
    fn initialize() -> Self {
        let temporary_directory = tempfile::tempdir()
            .expect("failed to initialize temporary directory");
        let temporary_directory_path = temporary_directory.path();
        temporary_directory_path.assert_is_directory_and_empty();
        Self { temporary_directory }
    }
    fn destroy(self) {
        self.temporary_directory
            .close()
            .expect("failed to destroy filesystem harness directory");
    }
}
impl AsPath for EmptyTree {
    fn as_path(&self) -> &Path {
        self.temporary_directory.path()
    }
}
