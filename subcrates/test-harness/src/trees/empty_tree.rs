use fs_more_test_harness_macros::fs_harness_tree;

use crate::assertable::AssertableRootDirectory;

#[fs_harness_tree]
pub struct EmptyTreeHarness {
    #[root]
    pub root: AssertableRootDirectory,
}
