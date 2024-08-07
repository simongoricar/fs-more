use fs_more::{
    error::FileError,
    file::{CollidingFileBehaviour, FileCopyFinished, FileCopyWithProgressOptions},
};
use fs_more_test_harness::{prelude::*, trees::structures::simple::SimpleTree};



#[test]
pub fn copy_file_with_progress_creates_an_identical_copy_and_reports_sensible_progress(
) -> TestResult {
    let harness = SimpleTree::initialize();


    let destination_file_path = harness.child_path("test-file.txt");
    destination_file_path.assert_not_exists();



    let expected_final_file_size_bytes = harness.yes.hello_world_txt.size_in_bytes();

    let mut last_bytes_finished_report: Option<u64> = None;
    let mut last_bytes_total_report: Option<u64> = None;


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.hello_world_txt.as_path(),
        &destination_file_path,
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Abort,
            ..Default::default()
        },
        |progress| {
            if let Some(last_bytes_finished) = last_bytes_finished_report {
                if progress.bytes_finished < last_bytes_finished {
                    panic!(
                        "invalid progress report: bytes_finished must never decrease \
                        (got {} -> {})",
                        last_bytes_finished, progress.bytes_finished
                    );
                }

                if let Some(last_bytes_total) = last_bytes_total_report {
                    if last_bytes_total != progress.bytes_total {
                        panic!(
                            "invalid progress report: bytes_total must never change \
                            (got {} -> {})",
                            last_bytes_total, progress.bytes_total
                        );
                    }
                }
            }

            last_bytes_finished_report = Some(progress.bytes_finished);
            last_bytes_total_report = Some(progress.bytes_total);
        },
    );


    assert_matches!(
        copy_result.unwrap(),
        FileCopyFinished::Created { bytes_copied }
        if bytes_copied == expected_final_file_size_bytes
    );


    let last_bytes_finished_report = last_bytes_finished_report.unwrap();
    let last_bytes_total_report = last_bytes_total_report.unwrap();

    assert_eq!(expected_final_file_size_bytes, last_bytes_finished_report);

    assert_eq!(expected_final_file_size_bytes, last_bytes_total_report);


    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();

    harness
        .yes
        .hello_world_txt
        .assert_initial_state_matches_other_file(&destination_file_path);


    harness.destroy();
    Ok(())
}


#[test]
pub fn copy_file_with_progress_errors_when_trying_to_copy_into_self() -> TestResult {
    let harness = SimpleTree::initialize();


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.hello_world_txt.as_path(),
        harness.yes.hello_world_txt.as_path(),
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if paths_equal_no_unc(&path, harness.yes.hello_world_txt.as_path())
    );


    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();


    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_with_progress_handles_case_insensitivity_properly() -> TestResult {
    let harness = SimpleTree::initialize();
    let is_fs_case_sensitive = detect_case_sensitivity_for_temp_dir();


    let source_file_size_bytes = harness.yes.no_bin.size_in_bytes();


    let destination_file_path = {
        let destination_file_name = harness
            .yes
            .no_bin
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_uppercase();

        harness
            .yes
            .no_bin
            .as_path()
            .with_file_name(destination_file_name)
    };


    if is_fs_case_sensitive {
        destination_file_path.assert_not_exists();
    } else {
        destination_file_path.assert_is_file_and_not_symlink();
    }


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.no_bin.as_path(),
        &destination_file_path,
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Abort,
            ..Default::default()
        },
        |_| {},
    );


    if is_fs_case_sensitive {
        assert_matches!(
            copy_result.unwrap(),
            FileCopyFinished::Created { bytes_copied }
            if bytes_copied == source_file_size_bytes
        );
    } else {
        assert_matches!(
            copy_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if paths_equal_no_unc(&path, harness.yes.no_bin.as_path())
                || paths_equal_no_unc(&path, &destination_file_path)
        );
    }


    harness.yes.no_bin.assert_unchanged_from_initial_state();

    destination_file_path.assert_is_file_and_not_symlink();
    harness
        .yes
        .no_bin
        .assert_initial_state_matches_other_file(&destination_file_path);


    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_with_progress_errors_when_trying_to_copy_into_self_even_when_more_complicated(
) -> TestResult {
    let harness = SimpleTree::initialize();
    let is_fs_case_sensitive = detect_case_sensitivity_for_temp_dir();


    let source_file_size_bytes = harness.yes.hello_world_txt.size_in_bytes();


    let destination_file_path = {
        let hello_world_uppercased_file_name = harness
            .yes
            .hello_world_txt
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_uppercase();

        harness
            .yes
            .as_path()
            .join("..")
            .join(harness.yes.as_path_relative_to_harness_root())
            .join(hello_world_uppercased_file_name)
    };


    if is_fs_case_sensitive {
        destination_file_path.assert_not_exists();
    } else {
        destination_file_path.assert_is_file_and_not_symlink();
    }


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.hello_world_txt.as_path(),
        &destination_file_path,
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Abort,
            ..Default::default()
        },
        |_| {},
    );


    if is_fs_case_sensitive {
        assert_matches!(
            copy_result.unwrap(),
            FileCopyFinished::Created { bytes_copied }
            if bytes_copied == source_file_size_bytes
        );
    } else {
        assert_matches!(
            copy_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if paths_equal_no_unc(&path, harness.yes.hello_world_txt.as_path())
                || paths_equal_no_unc(&path, &destination_file_path)
        );
    }


    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();

    harness
        .yes
        .hello_world_txt
        .assert_initial_state_matches_other_file(&destination_file_path);



    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_with_progress_overwrites_destination_file_when_behaviour_is_overwrite(
) -> TestResult {
    let harness = SimpleTree::initialize();

    let source_file_size_bytes = harness.yes.no_bin.size_in_bytes();


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.no_bin.as_path(),
        harness.yes.hello_world_txt.as_path(),
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap(),
        FileCopyFinished::Overwritten { bytes_copied }
        if bytes_copied == source_file_size_bytes
    );


    harness.yes.no_bin.assert_unchanged_from_initial_state();

    harness.yes.hello_world_txt.assert_is_file_and_not_symlink();
    harness
        .yes
        .no_bin
        .assert_initial_state_matches_other_file(harness.yes.hello_world_txt.as_path());



    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_with_progress_errors_on_existing_destination_file_when_behaviour_is_abort(
) -> TestResult {
    let harness = SimpleTree::initialize();


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.no_bin.as_path(),
        harness.yes.hello_world_txt.as_path(),
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Abort,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        FileError::DestinationPathAlreadyExists { path }
        if paths_equal_no_unc(&path, harness.yes.hello_world_txt.as_path())
    );


    harness.yes.no_bin.assert_unchanged_from_initial_state();
    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();


    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_with_progress_skips_existing_destination_file_when_behaviour_is_skip() -> TestResult
{
    let harness = SimpleTree::initialize();


    let copy_result = fs_more::file::copy_file_with_progress(
        harness.yes.hello_world_txt.as_path(),
        harness.yes.no_bin.as_path(),
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Skip,
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(copy_result.unwrap(), FileCopyFinished::Skipped);


    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();

    harness.yes.no_bin.assert_unchanged_from_initial_state();


    harness.destroy();
    Ok(())
}



/// Tests behaviour when copying "symlink to file A" to "A",
/// even when the overwriting behaviour is set. This operation must fail.
#[test]
pub fn copy_file_with_progress_errors_when_source_path_is_symlink_to_destination_file() -> TestResult
{
    let harness = SimpleTree::initialize();


    let source_symlink_path = harness.child_path("symlink");
    source_symlink_path.assert_not_exists();
    source_symlink_path.symlink_to_file(harness.yes.hello_world_txt.as_path());



    let copy_result = fs_more::file::copy_file_with_progress(
        &source_symlink_path,
        harness.yes.hello_world_txt.as_path(),
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if paths_equal_no_unc(&path, harness.yes.hello_world_txt.as_path())
    );


    harness.destroy();
    Ok(())
}



/// **On Windows**, creating symbolic links requires administrator privileges, unless Developer mode is enabled.
/// See [https://stackoverflow.com/questions/58038683/allow-mklink-for-a-non-admin-user].
#[test]
pub fn copy_file_with_progress_does_not_preserve_symlinks() -> TestResult {
    let harness = SimpleTree::initialize();


    let symlink_path = harness.child_path("symlink");
    symlink_path.assert_not_exists();
    symlink_path.symlink_to_file(harness.yes.no_bin.as_path());


    let symlink_destination_file_size_bytes = harness.yes.no_bin.as_path().size_in_bytes();


    let copy_destination_path = harness.child_path("destination-file");
    copy_destination_path.assert_not_exists();



    let copy_result = fs_more::file::copy_file_with_progress(
        &symlink_path,
        &copy_destination_path,
        FileCopyWithProgressOptions {
            colliding_file_behaviour: CollidingFileBehaviour::Abort,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap(),
        FileCopyFinished::Created { bytes_copied }
        if bytes_copied == symlink_destination_file_size_bytes
    );


    symlink_path
        .assert_is_valid_symlink_to_file_and_destination_matches(harness.yes.no_bin.as_path());

    copy_destination_path.assert_is_file_and_not_symlink();

    harness
        .yes
        .no_bin
        .assert_initial_state_matches_other_file(&copy_destination_path);


    harness.destroy();
    Ok(())
}
