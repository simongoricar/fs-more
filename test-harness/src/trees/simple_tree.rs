use fs_more_test_harness_macros::fs_harness_tree;
use once_cell::sync::Lazy;

use crate::{
    assertable::{AssertableDirectoryPath, AssertableFilePath, AssertableRootDirectory},
    lazy_generate_seeded_binary_data,
};


static BINARY_DATA_A: Lazy<Vec<u8>> = lazy_generate_seeded_binary_data!(1024 * 32, 2903489125012);

static BINARY_DATA_B: Lazy<Vec<u8>> = lazy_generate_seeded_binary_data!(1024 * 64, 2397591013122);


#[fs_harness_tree]
pub struct SimpleTreeHarness {
    #[root]
    pub root: AssertableRootDirectory,

    #[file(
        path = "binary_file_a.bin",
        content = BINARY_DATA_A.as_slice(),
    )]
    pub binary_file_a: AssertableFilePath,

    #[directory(path = "subdirectory_b")]
    pub subdirectory_b: AssertableDirectoryPath,

    #[file(
        path = "subdirectory_b/binary_file_b.bin",
        content = BINARY_DATA_B.as_slice(),
    )]
    pub binary_file_b: AssertableFilePath,
}
