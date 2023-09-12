use assert_fs::fixture::FixtureError;
use assert_matches::assert_matches;
use fs_more::{
    error::FileError,
    file::{FileCopyOptions, FileCopyWithProgressOptions},
};
use fs_more_test_harness::{
    assert_file_content_match,
    error::TestResult,
    DoubleFileHarness,
    SingleFileHarness,
};


/*
 * COPYING WITHOUT PROGRESS
 */

#[test]
pub fn copy_file() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let target_file_path = harness.file_path().with_file_name("test_file2.txt");

    let file_copy_result: Result<u64, FileError> = fs_more::file::copy_file(
        harness.file_path(),
        &target_file_path,
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    );

    assert!(
        file_copy_result.is_ok(),
        "failed to execute fs_more::file::copy_file: {}",
        file_copy_result.unwrap_err()
    );
    assert!(
        target_file_path.exists(),
        "fs_more::file::copy_file succeeded, but target file does not exist."
    );


    harness.destroy()?;

    Ok(())
}


#[test]
pub fn forbid_copy_into_self() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let file_copy_result = fs_more::file::copy_file(
        harness.file_path(),
        harness.file_path(),
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    );

    assert!(
        file_copy_result.is_err(),
        "fs_more::file::copy_file should have errored when trying to copy a file into itself"
    );
    let file_copy_err = file_copy_result.unwrap_err();
    assert_matches!(
        file_copy_err,
        FileError::SourceAndTargetAreTheSameFile,
        "fs_more::file::copy_file should have errored with SourceAndTargetAreTheSameFile, got {} instead",
        file_copy_err
    );

    assert!(
        harness.file_path().exists(),
        "fs_more::file::copy_file error, but the source file was still deleted."
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn forbid_case_insensitive_copy_into_self() -> Result<(), FixtureError> {
    let harness = SingleFileHarness::new()?;

    // Generates an upper-case version of the file name.
    let target_file_name = harness
        .file_path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_uppercase();
    let target_file_path = harness.file_path().with_file_name(target_file_name);

    let file_copy_result = fs_more::file::copy_file(
        harness.file_path(),
        target_file_path,
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    );

    assert!(
        file_copy_result.is_err(),
        "fs_more::file::copy_file should have errored when trying to copy a file into itself, \
        even when the case is different"
    );
    let file_copy_err = file_copy_result.unwrap_err();
    assert_matches!(
        file_copy_err,
        FileError::SourceAndTargetAreTheSameFile,
        "fs_more::file::copy_file should have errored with SourceAndTargetAreTheSameFile, got {} instead",
        file_copy_err
    );

    assert!(
        harness.file_path().exists(),
        "fs_more::file::copy_file error, but the source file was still deleted."
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn allow_move_overwriting_file_with_flag() -> TestResult<()> {
    let harness = DoubleFileHarness::new()?;

    let file_copy_result = fs_more::file::copy_file(
        harness.first_file_path(),
        harness.second_file_path(),
        FileCopyOptions {
            overwrite_existing: true,
            skip_existing: false,
        },
    );

    assert!(
        file_copy_result.is_ok(),
        "Failed to execute fs_more::file::copy_file: {}",
        file_copy_result.unwrap_err()
    );

    assert!(
        harness.second_file_path().exists(),
        "fs_more::file::copy_file succeeded, but target file does not exist."
    );

    assert_file_content_match!(
        harness.first_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::copy_file modified the source file"
    );
    // This `expected_first_file_contents` is intentional
    // as we just overwrote the second file.
    assert_file_content_match!(
        harness.second_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::copy_file failed to overwrite the second file"
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn forbid_move_overwriting_file_without_flag() -> TestResult<()> {
    let harness = DoubleFileHarness::new()?;

    let file_copy_result = fs_more::file::copy_file(
        harness.first_file_path(),
        harness.second_file_path(),
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    );

    assert!(
        file_copy_result.is_err(),
        "fs_more::file::copy_file returned {} instead of Err",
        file_copy_result.unwrap()
    );

    assert!(
        harness.first_file_path().exists(),
        "fs_more::file::copy_file failed (which is OK), but source file no longer exists."
    );
    assert!(
        harness.second_file_path().exists(),
        "fs_more::file::copy_file failed (which is OK), but target file no longer exists."
    );

    assert_file_content_match!(
        harness.first_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::copy_file modified the source file"
    );
    // We must not have overwritten the file this time (unlike in `allow_overwriting_file_with_flag`).
    assert_file_content_match!(
        harness.second_file_path(),
        DoubleFileHarness::expected_second_file_contents(),
        otherwise "fs_more::file::copy_file did not keep the target file intact"
    );


    harness.destroy()?;

    Ok(())
}

#[test]
pub fn skip_existing_target_file_move_with_flag() -> TestResult<()> {
    let harness = DoubleFileHarness::new()?;

    let file_copy_result = fs_more::file::copy_file(
        harness.first_file_path(),
        harness.second_file_path(),
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: true,
        },
    );

    assert!(
        file_copy_result.is_ok(),
        "fs_more::file::copy_file returned {} instead of Ok",
        file_copy_result.unwrap()
    );
    assert_eq!(
        file_copy_result.unwrap(),
        0,
        "fs_more::file::copy_file returned Ok, but copied non-zero bytes",
    );

    assert!(
        harness.first_file_path().exists(),
        "fs_more::file::copy_file returned Ok, but source file no longer exists."
    );
    assert!(
        harness.second_file_path().exists(),
        "fs_more::file::copy_file returned Ok, but target file no longer exists."
    );

    assert_file_content_match!(
        harness.first_file_path(),
        DoubleFileHarness::expected_first_file_contents(),
        otherwise "fs_more::file::copy_file modified the source file"
    );
    // We must not have overwritten the file (unlike in `allow_overwriting_file_with_flag`).
    assert_file_content_match!(
        harness.second_file_path(),
        DoubleFileHarness::expected_second_file_contents(),
        otherwise "fs_more::file::copy_file did not keep the target file intact"
    );


    harness.destroy()?;

    Ok(())
}



/*
 * COPYING WITH PROGRESS
 */


#[test]
pub fn copy_file_with_progress() -> TestResult<()> {
    let harness = SingleFileHarness::new()?;

    let target_file_path = harness.file_path().with_file_name("test_file2.txt");

    let target_file_size_bytes = harness.file_path().metadata()?.len();

    let mut last_bytes_copied = 0;
    let mut total_bytes = 0;

    let file_copy_result: Result<u64, FileError> =
        fs_more::file::copy_file_with_progress(
            harness.file_path(),
            &target_file_path,
            FileCopyWithProgressOptions {
                overwrite_existing: false,
                skip_existing: false,
                ..Default::default()
            },
            |progress| {
                last_bytes_copied = progress.bytes_finished;
                total_bytes = progress.bytes_total;
            },
        );

    assert!(
        file_copy_result.is_ok(),
        "failed to execute fs_more::file::copy_file_with_progress: {}.",
        file_copy_result.unwrap_err()
    );

    let bytes_copied = file_copy_result.unwrap();
    assert_eq!(
        bytes_copied, last_bytes_copied,
        "copy_file_with_progress failed to report some last writes \
        (return value and last progress update do not match)"
    );

    assert_eq!(
        bytes_copied, target_file_size_bytes,
        "copied vs real total bytes mismatch: the copy_file_with_progress return value \
        doesn't match the entire file size reported by the filesystem"
    );
    assert_eq!(
        bytes_copied, total_bytes,
        "copied vs total bytes mismatch: the copy_file_with_progress return value \
        doesn't match the entire file size reported by the same function"
    );

    assert!(
        harness.file_path().exists(),
        "copying succeeded, but source file has dissapeared."
    );
    assert!(
        target_file_path.exists(),
        "copying succeeded, but target file does not exist."
    );

    assert_file_content_match!(
        harness.file_path(),
        SingleFileHarness::expected_file_contents(),
        otherwise "copy_file_with_progress has tampered with the source file"
    );
    assert_file_content_match!(
        target_file_path,
        SingleFileHarness::expected_file_contents(),
        otherwise "copy_file_with_progress did not copy the file correctly"
    );


    harness.destroy()?;

    Ok(())
}
