use assert_fs::fixture::FixtureError;
use assert_matches::assert_matches;
use fs_more::{error::FileError, file::FileMoveOptions};
use fs_more_test_harness::{
    assert_file_content_match,
    error::TestResult,
    DoubleFileHarness,
    SingleFileHarness,
};

#[test]
pub fn move_file() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let target_file_path = harness.file_path().with_file_name("test_file2.txt");

    let file_copy_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.file_path(),
        &target_file_path,
        &FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_copy_result.is_ok(),
        "failed to execute fs_more::file::move_file: {}",
        file_copy_result.unwrap_err()
    );
    assert!(
        !harness.file_path().exists(),
        "fs_more::file::move_file succeeded, but source file still exists."
    );
    assert!(
        target_file_path.exists(),
        "fs_more::file::move_file succeeded, but the target file does not exist."
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn forbid_move_into_itself() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.file_path(),
        harness.file_path(),
        &FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_move_result.is_err(),
        "fs_more::file::move_file should have errored, but got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::SourceAndTargetAreTheSameFile,
        "fs_more::file::move_file should have errored with \
        SourceAndTargetAreTheSameFile, got {}.",
        move_err
    );

    assert!(
        harness.file_path().exists(),
        "fs_more::file::move_file errored (which is Ok), but source file is gone anyway."
    );

    assert_file_content_match!(
        harness.file_path(),
        SingleFileHarness::expected_file_contents(),
        otherwise "fs_more::file::move_file tampered with the file contents."
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn forbid_move_into_itself_with_overwrite_flag() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.file_path(),
        harness.file_path(),
        &FileMoveOptions {
            overwrite_existing: true,
        },
    );

    assert!(
        file_move_result.is_err(),
        "fs_more::file::move_file should have errored, but got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::SourceAndTargetAreTheSameFile,
        "fs_more::file::move_file should have errored with \
        SourceAndTargetAreTheSameFile, got {}.",
        move_err
    );

    assert!(
        harness.file_path().exists(),
        "fs_more::file::move_file errored (which is Ok), but source file is gone anyway."
    );

    assert_file_content_match!(
        harness.file_path(),
        SingleFileHarness::expected_file_contents(),
        otherwise "fs_more::file::move_file tampered with the file contents."
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn forbid_case_insensitive_move_into_itself() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let upper_case_file_name = harness
        .file_path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_uppercase();
    let target_file_path =
        harness.file_path().with_file_name(upper_case_file_name);

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.file_path(),
        target_file_path,
        &FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_move_result.is_err(),
        "fs_more::file::move_file should have errored, but got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::SourceAndTargetAreTheSameFile,
        "fs_more::file::move_file should have errored with \
        SourceAndTargetAreTheSameFile, got {}.",
        move_err
    );

    assert!(
        harness.file_path().exists(),
        "fs_more::file::move_file errored (which is Ok), but source file is gone anyway."
    );

    assert_file_content_match!(
        harness.file_path(),
        SingleFileHarness::expected_file_contents(),
        otherwise "fs_more::file::move_file tampered with the file contents."
    );


    harness.destroy()?;

    Ok(())
}


#[test]
pub fn allow_move_overwriting_target_file_with_flag() -> TestResult<()> {
    let harness = DoubleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.first_file_path(),
        harness.second_file_path(),
        &FileMoveOptions {
            overwrite_existing: true,
        },
    );

    assert!(
        file_move_result.is_ok(),
        "fs_more::file::move_file should have Ok-ed, but got {}.",
        file_move_result.unwrap_err()
    );

    let move_ok = file_move_result.unwrap();
    assert!(
        move_ok > 0,
        "fs_more::file::move_file should have returned a non-zero number of moved bytes, \
        but got {}.",
        move_ok
    );


    assert!(
        !harness.first_file_path().exists(),
        "source file still exists."
    );
    assert!(
        harness.second_file_path().exists(),
        "target file no longer exists."
    );

    assert_file_content_match!(
        harness.second_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::move_file did not overwrite second file correctly."
    );


    harness.destroy()?;

    Ok(())
}


#[test]
pub fn forbid_move_overwriting_target_file_without_flag() -> TestResult<()> {
    let harness = DoubleFileHarness::new()?;

    let file_move_result: Result<u64, FileError> = fs_more::file::move_file(
        harness.first_file_path(),
        harness.second_file_path(),
        &FileMoveOptions {
            overwrite_existing: false,
        },
    );

    assert!(
        file_move_result.is_err(),
        "fs_more::file::move_file should have errored, got {}.",
        file_move_result.unwrap()
    );

    let move_err = file_move_result.unwrap_err();
    assert_matches!(
        move_err,
        FileError::AlreadyExists,
        "fs_more::file::move_file should have returned AlreadyExists, got {}",
        move_err
    );


    assert!(
        harness.first_file_path().exists(),
        "source file no longer exists."
    );
    assert!(
        harness.second_file_path().exists(),
        "target file no longer exists."
    );

    assert_file_content_match!(
        harness.first_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::move_file modified the first file erroneously."
    );
    assert_file_content_match!(
        harness.second_file_path(),
        DoubleFileHarness::expected_second_file_contents(),
        otherwise "fs_more::file::move_file modified the second file erroneously."
    );


    harness.destroy()?;

    Ok(())
}
