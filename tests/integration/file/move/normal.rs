use assert_matches::assert_matches;
use fs_more::{
    error::FileError,
    file::{ExistingFileBehaviour, FileMoveFinished, FileMoveMethod, FileMoveOptions},
};
use fs_more_test_harness::{
    assertable::{
        r#trait::{AssertablePath, CaptureableFilePath, ManageablePath},
        AsPath,
    },
    case_sensitivity::detect_case_sensitivity_for_temp_dir,
    error::TestResult,
    tree_framework::{FileSystemHarness, FileSystemHarnessDirectory},
    trees::simple::SimpleTree,
};



#[test]
pub fn move_file_correctly_moves_the_file() -> TestResult {
    let harness = SimpleTree::initialize();

    let destination_file_path = harness.child_path("destination-file.txt");
    destination_file_path.assert_not_exists();

    harness.yes.no_bin.assert_is_file();
    let captured_before_move = harness.yes.no_bin.capture_with_content();


    let move_result = fs_more::file::move_file(
        harness.yes.no_bin.as_path(),
        &destination_file_path,
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert!(move_result.is_ok());


    harness.yes.no_bin.assert_not_exists();
    destination_file_path.assert_is_file();

    captured_before_move.assert_captured_state_matches_other_file(destination_file_path.as_path());


    harness.destroy();
    Ok(())
}


#[test]
pub fn move_file_errors_when_trying_to_copy_into_self() -> TestResult {
    let harness = SimpleTree::initialize();


    let bar_bin_captured = harness.yes.no_bin.capture_with_content();


    let move_result = fs_more::file::move_file(
        harness.yes.no_bin.as_path(),
        harness.yes.no_bin.as_path(),
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );


    assert_matches!(
        move_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.yes.no_bin.as_path()
    );

    harness.yes.no_bin.assert_is_file();
    bar_bin_captured.assert_unchanged();


    harness.destroy();
    Ok(())
}



#[test]
pub fn move_file_errors_when_trying_to_copy_into_self_even_with_overwrite_behaviour() -> TestResult
{
    let harness = SimpleTree::initialize();

    let bar_bin_captured = harness.yes.no_bin.capture_with_content();


    let move_result = fs_more::file::move_file(
        harness.yes.no_bin.as_path(),
        harness.yes.no_bin.as_path(),
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert_matches!(
        move_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.yes.no_bin.as_path()
    );


    harness.yes.no_bin.assert_is_file();
    bar_bin_captured.assert_unchanged();


    harness.destroy();
    Ok(())
}



#[test]
pub fn move_file_errors_when_trying_to_copy_into_case_insensitive_self() -> TestResult {
    let is_fs_case_sensitive = detect_case_sensitivity_for_temp_dir();
    let harness = SimpleTree::initialize();


    let hello_world_uppercased_file_name = harness
        .yes
        .hello_world_txt
        .as_path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_uppercase();

    let hello_world_uppercased_file_path = harness
        .yes
        .hello_world_txt
        .as_path()
        .with_file_name(hello_world_uppercased_file_name);


    let captured_hello_world = harness.yes.hello_world_txt.capture_with_content();


    if is_fs_case_sensitive {
        hello_world_uppercased_file_path.assert_not_exists();
    } else {
        hello_world_uppercased_file_path.assert_is_file();
    }


    let file_move_result = fs_more::file::move_file(
        harness.yes.hello_world_txt.as_path(),
        &hello_world_uppercased_file_path,
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );


    if is_fs_case_sensitive {
        assert!(
            file_move_result.is_ok(),
            "move_file should have ok-ed (on case-sensitive filesystem), got {:?}",
            file_move_result.unwrap_err(),
        );

        harness.yes.hello_world_txt.assert_not_exists();

        hello_world_uppercased_file_path.assert_is_file();
        captured_hello_world
            .assert_captured_state_matches_other_file(&hello_world_uppercased_file_path);
    } else {
        assert!(
            file_move_result.is_err(),
            "move_file should have errored (on case-insensitive filesystem), got {:?}.",
            file_move_result.unwrap()
        );

        assert_matches!(
            file_move_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if path == hello_world_uppercased_file_path.as_path() || path == harness.yes.hello_world_txt.as_path()
        );

        captured_hello_world.assert_unchanged();

        hello_world_uppercased_file_path.assert_is_file();
        captured_hello_world
            .assert_captured_state_matches_other_file(harness.yes.hello_world_txt.as_path());
    }


    harness.destroy();
    Ok(())
}



#[test]
pub fn move_file_errors_when_source_is_symlink_to_destination() -> TestResult {
    let harness = SimpleTree::initialize();


    let symlink_path = harness.child_path("some-symlink.txt");
    symlink_path.assert_not_exists();

    symlink_path.symlink_to_file(harness.yes.hello_world_txt.as_path());
    symlink_path.assert_is_symlink_to_file();


    let captured_hello_world_txt = harness.yes.hello_world_txt.capture_with_content();


    let move_result = fs_more::file::move_file(
        &symlink_path,
        harness.yes.hello_world_txt.as_path(),
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert_matches!(
        move_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.yes.hello_world_txt.as_path() || path == symlink_path
    );


    symlink_path.assert_is_symlink_to_file();
    captured_hello_world_txt.assert_unchanged();


    harness.destroy();
    Ok(())
}



#[test]
pub fn move_file_overwrites_destination_file_when_behaviour_is_overwrite() -> TestResult {
    let harness = SimpleTree::initialize();

    let captured_source_file = harness.yes.hello_world_txt.capture_with_content();
    let source_file_size = harness.yes.hello_world_txt.size_in_bytes();


    let move_result = fs_more::file::move_file(
        harness.yes.hello_world_txt.as_path(),
        harness.yes.no_bin.as_path(),
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert_matches!(
        move_result.unwrap(),
        FileMoveFinished::Overwritten { bytes_copied, .. }
        if bytes_copied == source_file_size
    );


    harness.yes.hello_world_txt.assert_not_exists();
    captured_source_file.assert_captured_state_matches_other_file(harness.yes.no_bin.as_path());


    harness.destroy();
    Ok(())
}



#[test]
pub fn move_file_errors_on_existing_destination_file_when_behaviour_is_abort() -> TestResult {
    let harness = SimpleTree::initialize();

    let source_file_captured = harness.yes.hello_world_txt.capture_with_content();
    let destination_file_captured = harness.yes.no_bin.capture_with_content();


    let move_result = fs_more::file::move_file(
        harness.yes.hello_world_txt.as_path(),
        harness.yes.no_bin.as_path(),
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert_matches!(
        move_result.unwrap_err(),
        FileError::DestinationPathAlreadyExists { path }
        if path == harness.yes.no_bin.as_path()
    );


    source_file_captured.assert_unchanged();
    destination_file_captured.assert_unchanged();


    harness.destroy();
    Ok(())
}




/// **On Windows**, creating symbolic links requires administrator privileges, unless Developer mode is enabled.
/// See <https://stackoverflow.com/questions/58038683/allow-mklink-for-a-non-admin-user>.
#[test]
pub fn move_file_may_preserve_symlinks_when_moving_by_rename() -> TestResult {
    let harness = SimpleTree::initialize();


    let symlink_destination_file_size_bytes = harness.yes.hello_world_txt.size_in_bytes();
    let captured_symlink_destination_file = harness.yes.hello_world_txt.capture_with_content();


    let symlink_file_path = harness.child_path("some-symlink.txt");
    symlink_file_path.assert_not_exists();

    symlink_file_path.symlink_to_file(harness.yes.hello_world_txt.as_path());


    let symlink_moved_file_path = harness.child_path("some-symlink.moved.txt");
    symlink_moved_file_path.assert_not_exists();


    let finished_move = fs_more::file::move_file(
        &symlink_file_path,
        &symlink_moved_file_path,
        FileMoveOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    )
    .unwrap();


    match finished_move {
        FileMoveFinished::Created {
            bytes_copied,
            method,
        } => match method {
            FileMoveMethod::Rename => {
                // The symlink was preserved.
                symlink_moved_file_path.assert_is_symlink_to_file_and_destination_matches(
                    harness.yes.hello_world_txt.as_path(),
                );
            }
            FileMoveMethod::CopyAndDelete => {
                // The symlink was not preserved.
                assert_eq!(bytes_copied, symlink_destination_file_size_bytes);
                symlink_moved_file_path.assert_is_symlink_to_file_and_destination_matches(
                    harness.yes.hello_world_txt.as_path(),
                );
            }
        },
        _ => panic!("move_file did not abort, even though ExistingFileBehaviour::Abort was set"),
    }


    symlink_file_path.assert_not_exists();
    captured_symlink_destination_file.assert_unchanged();


    harness.destroy();
    Ok(())
}
