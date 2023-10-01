use assert_matches::assert_matches;
use fs_more::{error::FileError, file::FileMoveOptions};
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::SimpleFileHarness,
};

#[test]
pub fn move_file() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let target_file = AssertableFilePath::from_path_pure(
        harness.test_file.path().with_file_name("test_file2.txt"),
    );

    let file_copy_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.test_file.path(),
        target_file.path(),
        FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_copy_result.is_ok(),
        "failed to execute move_file: {}",
        file_copy_result.unwrap_err()
    );

    harness.test_file.assert_not_exists();

    target_file.assert_exists();
    target_file.assert_content_matches_expected_value_of_assertable(&harness.test_file);


    harness.destroy()?;
    Ok(())
}


#[test]
pub fn forbid_move_into_itself() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.foo_bar.path(),
        harness.foo_bar.path(),
        FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, but got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::SourceAndTargetAreTheSameFile,
        "move_file should have errored with \
        SourceAndTargetAreTheSameFile, got {}.",
        move_err
    );

    harness.foo_bar.assert_exists();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}

#[test]
pub fn forbid_move_into_itself_with_overwrite_flag() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.foo_bar.path(),
        harness.foo_bar.path(),
        FileMoveOptions {
            overwrite_existing: true,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, but got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::SourceAndTargetAreTheSameFile,
        "move_file should have errored with SourceAndTargetAreTheSameFile, got {}.",
        move_err
    );

    harness.foo_bar.assert_exists();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}

#[test]
pub fn forbid_case_insensitive_move_into_itself() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let upper_case_file_name = harness
        .foo_bar
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_uppercase();

    let target_file = AssertableFilePath::from_path_pure(
        harness.foo_bar.path().with_file_name(upper_case_file_name),
    );

    #[cfg(unix)]
    target_file.assert_not_exists();

    #[cfg(windows)]
    target_file.assert_exists();

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.foo_bar.path(),
        target_file.path(),
        FileMoveOptions {
            overwrite_existing: false,
        },
    );

    #[cfg(unix)]
    {
        assert!(
            file_move_result.is_ok(),
            "move_file should have ok-ed (on unix), but got {}",
            file_move_result.unwrap_err(),
        );

        target_file.assert_exists();
        harness.foo_bar.assert_not_exists();
    }

    #[cfg(windows)]
    {
        assert!(
            file_move_result.is_err(),
            "move_file should have errored, but got {}.",
            file_move_result.unwrap()
        );

        let move_err = file_move_result.unwrap_err();
        assert_matches!(
            move_err,
            FileError::SourceAndTargetAreTheSameFile,
            "move_file should have errored with SourceAndTargetAreTheSameFile, got {}.",
            move_err
        );

        target_file.assert_exists();

        harness.foo_bar.assert_exists();
        harness.foo_bar.assert_content_unchanged();
    }



    harness.destroy()?;
    Ok(())
}


#[test]
pub fn allow_move_overwriting_target_file_with_flag() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.test_file.path(),
        harness.foo_bar.path(),
        FileMoveOptions {
            overwrite_existing: true,
        },
    );

    assert!(
        file_move_result.is_ok(),
        "move_file should have Ok-ed, but got {}.",
        file_move_result.unwrap_err()
    );

    let move_ok = file_move_result.unwrap();
    assert_eq!(
        harness.test_file.expected_content_unchecked().len(),
        move_ok as usize,
        "move_file did not return the precise amount of moved bytes"
    );

    harness.test_file.assert_not_exists();
    harness.foo_bar.assert_exists();

    harness
        .foo_bar
        .assert_content_matches_expected_value_of_assertable(&harness.test_file);


    harness.destroy()?;
    Ok(())
}


#[test]
pub fn forbid_move_overwriting_target_file_without_flag() -> TestResult<()> {
    let harness = SimpleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.test_file.path(),
        harness.foo_bar.path(),
        FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::AlreadyExists,
        "move_file should have returned AlreadyExists, got {}",
        move_err
    );

    harness.test_file.assert_exists();
    harness.foo_bar.assert_exists();

    harness.test_file.assert_content_unchanged();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}
