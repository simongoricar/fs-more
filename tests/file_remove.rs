use assert_matches::assert_matches;
use fs_more::error::FileRemoveError;
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::SimpleFileHarness,
};

#[test]
pub fn remove_file() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let removal_result = fs_more::file::remove_file(harness.test_file.path());


    assert!(
        removal_result.is_ok(),
        "failed to remove file: expected Ok, got {}",
        removal_result.unwrap_err()
    );

    harness.test_file.assert_not_exists();
    harness.foo_bar.assert_exists();

    harness.destroy()?;
    Ok(())
}


#[test]
pub fn fail_file_removal_when_it_doesnt_exist() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let non_existent_file = AssertableFilePath::from_path(
        harness
            .foo_bar
            .path()
            .with_file_name("random_nonexistent_file.md"),
    );
    non_existent_file.assert_not_exists();

    let removal_result = fs_more::file::remove_file(non_existent_file.path());


    assert!(
        removal_result.is_err(),
        "failed to error on file removal: expected Err, got Ok"
    );

    let removal_err = removal_result.unwrap_err();

    assert_matches!(
        removal_err,
        FileRemoveError::NotFound,
        "expected NotFound, got {}",
        removal_err
    );


    harness.destroy()?;
    Ok(())
}
