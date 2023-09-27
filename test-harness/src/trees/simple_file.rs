use fs_more_test_harness_derive::fs_harness_tree;

use crate::assertable::{AssertableFilePath, AssertableRootDirectory};

const FIRST_FILE_CONTENTS: &str = "This is the first file.";
const SECOND_FILE_CONTENTS: &str = "This is the second file.";

#[fs_harness_tree]
pub struct SimpleFileHarness {
    #[root]
    pub root: AssertableRootDirectory,

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
