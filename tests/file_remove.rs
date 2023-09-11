use assert_matches::assert_matches;
use fs_more::error::FileRemoveError;
use fs_more_test_harness::{error::TestResult, SingleFileHarness};

#[test]
pub fn remove_file() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let removal_result = fs_more::file::remove_file(harness.file_path());


    assert!(
        removal_result.is_ok(),
        "failed to remove file: expected Ok, got {}",
        removal_result.unwrap_err()
    );

    assert!(
        !harness.file_path().exists(),
        "remove_file succeeded, but the file still exists"
    );

    harness.destroy()?;

    Ok(())
}

#[test]
pub fn fail_file_removal_when_it_doesnt_exist() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let some_random_non_existent_file_path =
        harness.file_path().with_file_name("asdio32f.txt");

    let removal_result =
        fs_more::file::remove_file(&some_random_non_existent_file_path);


    assert!(
        removal_result.is_err(),
        "failed to fail at file removal: expected Err, got Ok"
    );

    let removal_err = removal_result.unwrap_err();

    assert_matches!(
        removal_err,
        FileRemoveError::NotFound,
        "expected NotFound error, got {}",
        removal_err
    );

    assert!(
        harness.file_path().exists(),
        "remove_file failed (which is Ok), but a completed unrelated file was removed"
    );
    assert!(
        !some_random_non_existent_file_path.exists(),
        "remove_file failed (which is Ok), but a completed unrelated file was created"
    );

    harness.destroy()?;

    Ok(())
}
