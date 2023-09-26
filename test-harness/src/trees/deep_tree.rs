use fs_more_test_harness_derive::fs_harness_tree;
use once_cell::sync::Lazy;

use crate::{
    assertable::{
        AssertableDirectoryPath,
        AssertableFilePath,
        AssertableRootDirectory,
    },
    lazy_generate_seeded_binary_data,
};


static BINARY_DATA_A: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 32, 2903489125012);

static BINARY_DATA_B: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 64, 2397591013122);

static BINARY_DATA_C: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 128, 394590111123);

static BINARY_DATA_D: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 256, 569119922498906);

static BINARY_DATA_E: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 16, 11235112229834);

static BINARY_DATA_F: Lazy<Vec<u8>> =
    lazy_generate_seeded_binary_data!(1024 * 1024, 34901111222);


#[fs_harness_tree]
pub struct DeepTreeHarness {
    #[root]
    pub root: AssertableRootDirectory,

    #[file(
        path = "file_a.bin",
        content = BINARY_DATA_A.as_slice(),
    )]
    pub file_a: AssertableFilePath,

    #[directory(path = "dir_foo")]
    pub dir_foo: AssertableDirectoryPath,

    // Empty directory.
    #[directory(path = "dir_foo2")]
    pub dir_foo_2: AssertableDirectoryPath,

    // Empty directory.
    #[directory(path = "dir_foo3")]
    pub dir_foo_3: AssertableDirectoryPath,

    #[file(
        path = "dir_foo/file_b.bin",
        content = BINARY_DATA_B.as_slice(),
    )]
    pub file_b: AssertableFilePath,

    #[directory(path = "dir_foo/dir_bar")]
    pub dir_bar: AssertableDirectoryPath,

    #[file(
        path = "dir_foo/dir_bar/file_c.bin",
        content = BINARY_DATA_C.as_slice(),
    )]
    pub file_c: AssertableFilePath,

    #[directory(path = "dir_foo/dir_bar/hello")]
    pub dir_hello: AssertableDirectoryPath,

    #[directory(path = "dir_foo/dir_bar/hello/world")]
    pub dir_world: AssertableDirectoryPath,

    #[file(
        path = "dir_foo/dir_bar/hello/world/file_d.bin",
        content = BINARY_DATA_D.as_slice(),
    )]
    pub file_d: AssertableFilePath,

    #[file(
        path = "dir_foo/dir_bar/hello/world/file_e.bin",
        content = BINARY_DATA_E.as_slice(),
    )]
    pub file_e: AssertableFilePath,

    #[file(
        path = "dir_foo/dir_bar/hello/world/file_f.bin",
        content = BINARY_DATA_F.as_slice(),
    )]
    pub file_f: AssertableFilePath,
}
