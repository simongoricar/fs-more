use fs_more_test_harness_derive::FilesystemTreeHarness;

use crate::assertable::{AssertableFilePath, AssertableRootPath};

const FIRST_FILE_CONTENTS: &str = "This is the first file.";
const SECOND_FILE_CONTENTS: &str = "This is the second file.";

#[derive(FilesystemTreeHarness)]
pub struct SimpleFileHarness {
    #[root]
    root: AssertableRootPath,

    #[file(
        path = "test_file.txt",
        content = FIRST_FILE_CONTENTS.as_bytes(),
    )]
    pub test_file: AssertableFilePath,

    #[file(
        path = "foo_bar.txt",
        content = SECOND_FILE_CONTENTS.as_bytes(),
    )]
    pub foo_bar: AssertableFilePath,
}
