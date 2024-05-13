use assert_matches::assert_matches;
use fs_more::{
    error::FileError,
    file::{ExistingFileBehaviour, MoveFileFinished, MoveFileMethod, MoveFileOptions},
};
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::{SimpleFileHarness, SimpleTreeHarness},
};



#[test]
pub fn move_file() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let target_file =
        AssertableFilePath::from_path(harness.test_file.path().with_file_name("test_file2.txt"));

    let file_copy_result = fs_more::file::move_file(
        harness.test_file.path(),
        target_file.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
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
pub fn forbid_move_into_itself() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let file_move_result = fs_more::file::move_file(
        harness.foo_bar.path(),
        harness.foo_bar.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, but got {:?}.",
        file_move_result.unwrap()
    );

    assert_matches!(
        file_move_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.foo_bar.path()
    );

    harness.foo_bar.assert_exists();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}



#[test]
pub fn forbid_move_into_itself_with_overwrite_flag() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let file_move_result = fs_more::file::move_file(
        harness.foo_bar.path(),
        harness.foo_bar.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, but got {:?}.",
        file_move_result.unwrap()
    );

    assert_matches!(
        file_move_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.foo_bar.path()
    );

    harness.foo_bar.assert_exists();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}



#[test]
pub fn forbid_case_insensitive_move_into_itself() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let upper_case_file_name = harness
        .foo_bar
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_uppercase();

    let target_file =
        AssertableFilePath::from_path(harness.foo_bar.path().with_file_name(upper_case_file_name));

    #[cfg(unix)]
    target_file.assert_not_exists();

    #[cfg(windows)]
    target_file.assert_exists();

    let file_move_result = fs_more::file::move_file(
        harness.foo_bar.path(),
        target_file.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    #[cfg(unix)]
    {
        assert!(
            file_move_result.is_ok(),
            "move_file should have ok-ed (on unix), but got {:?}",
            file_move_result.unwrap_err(),
        );

        target_file.assert_exists();
        harness.foo_bar.assert_not_exists();
    }

    #[cfg(windows)]
    {
        assert!(
            file_move_result.is_err(),
            "move_file should have errored (on windows), but got {:?}.",
            file_move_result.unwrap()
        );

        assert_matches!(
            file_move_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if path == target_file.path() || path == harness.foo_bar.path()
        );

        target_file.assert_exists();

        harness.foo_bar.assert_exists();
        harness.foo_bar.assert_content_unchanged();
    }



    harness.destroy()?;
    Ok(())
}



#[test]
pub fn forbid_move_file_when_source_is_symlink_to_destination() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let test_symlink =
        AssertableFilePath::from_path(harness.root.child_path("symlink-test-file.txt"));
    test_symlink.assert_not_exists();
    test_symlink
        .symlink_to_file(harness.test_file.path())
        .unwrap();
    test_symlink.assert_is_symlink_to_file();

    let copy_result = fs_more::file::move_file(
        test_symlink.path(),
        harness.test_file.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );


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



#[test]
pub fn allow_move_overwriting_target_file_with_flag() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let file_move_result = fs_more::file::move_file(
        harness.test_file.path(),
        harness.foo_bar.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert!(
        file_move_result.is_ok(),
        "move_file should have Ok-ed, but got {}.",
        file_move_result.unwrap_err()
    );

    assert_matches!(
        file_move_result.unwrap(),
        MoveFileFinished::Overwritten { bytes_copied, .. }
        if bytes_copied == harness.test_file.expected_content_unchecked().len() as u64
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
pub fn forbid_move_overwriting_target_file_without_flag() -> TestResult {
    let harness = SimpleFileHarness::new()?;

    let file_move_result = fs_more::file::move_file(
        harness.test_file.path(),
        harness.foo_bar.path(),
        MoveFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert!(
        file_move_result.is_err(),
        "move_file should have errored, got {:?}.",
        file_move_result.unwrap()
    );

    assert_matches!(
        file_move_result.unwrap_err(),
        FileError::DestinationPathAlreadyExists { path }
        if path == harness.foo_bar.path()
    );

    harness.test_file.assert_exists();
    harness.foo_bar.assert_exists();

    harness.test_file.assert_content_unchanged();
    harness.foo_bar.assert_content_unchanged();


    harness.destroy()?;
    Ok(())
}



/// **On Windows**, creating symbolic links requires administrator privileges, unless Developer mode is enabled.
/// See <https://stackoverflow.com/questions/58038683/allow-mklink-for-a-non-admin-user>.
#[test]
pub fn move_file_symlink_behaviour() -> TestResult {
    let harness = SimpleTreeHarness::new()?;

    let symlinked_file = AssertableFilePath::from_path(harness.root.child_path("my-symlink.txt"));
    symlinked_file.assert_not_exists();
    symlinked_file.symlink_to_file(harness.binary_file_a.path())?;
    symlinked_file.assert_is_symlink();

    let real_file_size_in_bytes = symlinked_file.file_size_in_bytes()?;

    let target_file =
        AssertableFilePath::from_path(harness.root.child_path("my-moved-symlink.txt"));
    target_file.assert_not_exists();


    let finished_move = fs_more::file::move_file(
        symlinked_file.path(),
        target_file.path(),
        MoveFileOptions::default(),
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
