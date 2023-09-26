// TODO Add directory copy tests.

use fs_more_test_harness::{error::TestResult, trees::DeepTreeHarness};

#[test]
pub fn copy_directory() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;

    // TODO

    // fs_more::directory::copy_directory(
    //        harness.
    // );


    harness.destroy()?;
    Ok(())
}
