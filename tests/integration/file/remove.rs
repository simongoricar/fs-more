use assert_matches::assert_matches;
use fs_more::error::FileRemoveError;
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::SimpleFileHarness,
};



#[test]
pub fn remove_file() -> TestResult {
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
pub fn remove_file_does_not_follow_symlinks() -> TestResult {
    let harness = SimpleFileHarness::new()?;
    let second_harness = SimpleFileHarness::new()?;

    harness.test_file.remove()?;
    harness.test_file.assert_not_exists();

    second_harness.test_file.assert_is_file();
    harness
        .test_file
        .symlink_to_file(second_harness.test_file.path())?;

    harness.test_file.assert_is_symlink_to_file();


    fs_more::file::remove_file(harness.test_file.path()).unwrap();

    harness.test_file.assert_not_exists();
    second_harness.test_file.assert_is_file();

    harness.destroy()?;
    Ok(())
}



#[test]
pub fn fail_file_removal_when_it_doesnt_exist() -> TestResult {
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

    assert_matches!(
        removal_result.unwrap_err(),
        FileRemoveError::NotFound { path }
        if path == non_existent_file.path()
    );


    harness.destroy()?;
    Ok(())
}
