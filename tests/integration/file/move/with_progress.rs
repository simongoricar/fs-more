use assert_matches::assert_matches;
use fs_more::{
    error::FileError,
    file::{
        ExistingFileBehaviour,
        FileProgress,
        MoveFileFinished,
        MoveFileMethod,
        MoveFileWithProgressOptions,
    },
};
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::{SimpleFileHarness, SimpleTreeHarness},
};



#[test]
pub fn move_file_with_progress() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let source_file_size_bytes = harness.test_file.path().metadata().unwrap().len();
    let target_file =
        AssertableFilePath::from_path(harness.test_file.path().with_file_name("test_file2.txt"));

    let mut last_progress: Option<FileProgress> = None;

    let file_copy_result = fs_more::file::move_file_with_progress(
        harness.test_file.path(),
        target_file.path(),
        MoveFileWithProgressOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            ..Default::default()
        },
        |progress| {
            if let Some(previous_progress) = last_progress.as_ref() {
                assert!(progress.bytes_finished >= previous_progress.bytes_finished);
            }

            last_progress = Some(progress.clone());
        },
    );

    let last_progress = last_progress.unwrap();

    assert_eq!(
        last_progress.bytes_finished,
        source_file_size_bytes
    );
    assert_eq!(last_progress.bytes_total, source_file_size_bytes);

    assert!(
        file_copy_result.is_ok(),
        "failed to execute move_file_with_progress: {}",
        file_copy_result.unwrap_err()
    );

    harness.test_file.assert_not_exists();

    target_file.assert_exists();
    target_file.assert_content_matches_expected_value_of_assertable(&harness.test_file);


    harness.destroy()?;
    Ok(())
}



/// **On Windows**, creating symbolic links requires administrator privileges, unless Developer mode is enabled.
/// See <https://stackoverflow.com/questions/58038683/allow-mklink-for-a-non-admin-user>.
#[test]
pub fn move_file_with_progress_symlink_behaviour() -> TestResult {
    let harness = SimpleTreeHarness::new()?;

    let symlinked_file = AssertableFilePath::from_path(harness.root.child_path("my-symlink.txt"));
    symlinked_file.assert_not_exists();
    symlinked_file.symlink_to_file(harness.binary_file_a.path())?;
    symlinked_file.assert_is_symlink();

    let real_file_size_in_bytes = symlinked_file.file_size_in_bytes()?;

    let target_file =
        AssertableFilePath::from_path(harness.root.child_path("my-moved-symlink.txt"));
    target_file.assert_not_exists();


    let finished_move = fs_more::file::move_file_with_progress(
        symlinked_file.path(),
        target_file.path(),
        MoveFileWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();

    match finished_move {
        MoveFileFinished::Created {
            bytes_copied,
            method,
        } => match method {
            MoveFileMethod::Rename => {
                // The symlink was preserved in this case.
                target_file.assert_is_symlink_to_file();
            }
            MoveFileMethod::CopyAndDelete => {
                // The symlink was not preserved.
                assert_eq!(bytes_copied, real_file_size_in_bytes);
                target_file.assert_is_file();
            }
        },
        _ => {
            panic!("move_file should have created a destination file");
        }
    }


    symlinked_file.assert_not_exists();
    harness.binary_file_a.assert_content_unchanged();
    target_file.assert_content_matches_file(harness.binary_file_a.path());

    assert_eq!(
        real_file_size_in_bytes,
        target_file.file_size_in_bytes()?
    );

    harness.destroy()?;
    Ok(())
}



#[test]
pub fn forbid_move_file_with_progress_when_source_is_symlink_to_target() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let test_symlink =
        AssertableFilePath::from_path(harness.root.child_path("symlink-test-file.txt"));
    test_symlink.assert_not_exists();
    test_symlink
        .symlink_to_file(harness.test_file.path())
        .unwrap();
    test_symlink.assert_is_symlink_to_file();

    let mut last_progress: Option<FileProgress> = None;

    let copy_result = fs_more::file::move_file_with_progress(
        test_symlink.path(),
        harness.test_file.path(),
        MoveFileWithProgressOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
            ..Default::default()
        },
        |progress| {
            last_progress = Some(progress.clone());
        },
    );

    assert!(last_progress.is_none());


    assert_matches!(
        copy_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.test_file.path()
    );


    test_symlink.assert_is_symlink_to_file();
    harness.test_file.assert_is_file();

    harness.destroy()?;
    Ok(())
}
