use fs_more_test_harness_derive::FilesystemTreeHarness;

use crate::assertable::{AssertableFilePath, AssertableRootPath};

const SINGLE_FILE_CONTENTS: &str = "This is the first file.";

#[derive(FilesystemTreeHarness)]
pub struct SimpleFileHarness {
    #[root]
    root: AssertableRootPath,

    #[file(
        path = "test_file.txt",
        content = SINGLE_FILE_CONTENTS.as_bytes(),
    )]
    pub single_file: AssertableFilePath,
}
