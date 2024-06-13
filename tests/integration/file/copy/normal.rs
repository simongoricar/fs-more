use assert_matches::assert_matches;
use fs_more::{
    error::FileError,
    file::{CopyFileFinished, CopyFileOptions, ExistingFileBehaviour},
};
use fs_more_test_harness::{
    assertable::{
        r#trait::{AssertablePath, CaptureableFilePath, ManageablePath},
        AsPath,
    },
    error::TestResult,
    is_temporary_directory_case_sensitive,
    tree_framework::{AssertableInitialFileCapture, FileSystemHarness, FileSystemHarnessDirectory},
    trees::simple::SimpleTree,
};



#[test]
pub fn copy_file_creates_an_identical_copy() -> TestResult {
    let harness = SimpleTree::initialize();


    let destination_file_path = harness.child_path("test-file.txt");
    destination_file_path.assert_not_exists();


    let captured_source_file = harness.yes.hello_world_txt.capture_with_content();
    let source_file_size_bytes = harness.yes.hello_world_txt.size_in_bytes();


    let copy_result = fs_more::file::copy_file(
        harness.yes.hello_world_txt.as_path(),
        &destination_file_path,
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert_matches!(
        copy_result.unwrap(),
        CopyFileFinished::Created { bytes_copied }
        if bytes_copied == source_file_size_bytes
    );


    destination_file_path.assert_is_file_and_not_symlink();
    captured_source_file.assert_unchanged();

    captured_source_file.assert_captured_state_matches_other_file(&destination_file_path);


    harness.destroy();
    Ok(())
}


#[test]
pub fn copy_file_errors_when_trying_to_copy_into_self() -> TestResult {
    let harness = SimpleTree::initialize();

    let captured_file = harness.yes.no_bin.capture_with_content();


    let copy_result = fs_more::file::copy_file(
        harness.yes.no_bin.as_path(),
        harness.yes.no_bin.as_path(),
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );

    assert_matches!(
        copy_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.yes.no_bin.as_path()
    );

    captured_file.assert_unchanged();


    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_handles_case_insensitivity_properly() -> TestResult {
    let harness = SimpleTree::initialize();
    let is_fs_case_sensitive = is_temporary_directory_case_sensitive();


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



    if is_fs_case_sensitive {
        hello_world_uppercased_file_path.assert_not_exists();
    } else {
        hello_world_uppercased_file_path.assert_is_file();
    }


    let copy_result = fs_more::file::copy_file(
        harness.yes.hello_world_txt.as_path(),
        &hello_world_uppercased_file_path,
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );


    if is_fs_case_sensitive {
        assert_matches!(
            copy_result.unwrap(),
            CopyFileFinished::Created { .. },
            "copy_file should have created a file (on case-sensitive filesystem) \
            when trying to copy a file into itself, even when the case is different"
        );
    } else {
        assert_matches!(
            copy_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if path == hello_world_uppercased_file_path.as_path() || path == harness.yes.hello_world_txt.as_path()
        );
    }

    harness
        .yes
        .hello_world_txt
        .assert_unchanged_from_initial_state();

    hello_world_uppercased_file_path.assert_is_file_and_not_symlink();
    harness
        .yes
        .hello_world_txt
        .assert_initial_state_matches_other_file(&hello_world_uppercased_file_path);


    harness.destroy();
    Ok(())
}



#[test]
pub fn copy_file_errors_when_trying_to_copy_into_self_even_when_more_complicated() -> TestResult {
    let harness = SimpleTree::initialize();
    let is_fs_case_sensitive = is_temporary_directory_case_sensitive();


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
            .join(harness.yes.as_path().file_name().unwrap())
            .join(hello_world_uppercased_file_name)
    };

    if is_fs_case_sensitive {
        destination_file_path.assert_not_exists();
    } else {
        destination_file_path.assert_is_file_and_not_symlink();
    }


    let copy_result = fs_more::file::copy_file(
        harness.yes.hello_world_txt.as_path(),
        &destination_file_path,
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );


    if is_fs_case_sensitive {
        assert_matches!(copy_result.unwrap(), CopyFileFinished::Created { .. });
    } else {
        assert_matches!(
            copy_result.unwrap_err(),
            FileError::SourceAndDestinationAreTheSame { path }
            if path == harness.yes.hello_world_txt.as_path() || path == destination_file_path
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
pub fn copy_file_overwrites_destination_file_when_behaviour_is_overwrite() -> TestResult {
    let harness = SimpleTree::initialize();

    let source_file_size_bytes = harness.yes.no_bin.size_in_bytes();


    let copy_result = fs_more::file::copy_file(
        harness.yes.no_bin.as_path(),
        harness.yes.hello_world_txt.as_path(),
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );


    assert_matches!(
        copy_result.unwrap(),
        CopyFileFinished::Overwritten { bytes_copied }
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
pub fn copy_file_errors_on_existing_destination_file_when_behaviour_is_abort() -> TestResult {
    let harness = SimpleTree::initialize();


    let copy_result = fs_more::file::copy_file(
        harness.yes.no_bin.as_path(),
        harness.yes.hello_world_txt.as_path(),
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );

    assert_matches!(
        copy_result.unwrap_err(),
        FileError::DestinationPathAlreadyExists { path }
        if path == harness.yes.hello_world_txt.as_path()
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
pub fn copy_file_skips_existing_destination_file_when_behaviour_is_skip() -> TestResult {
    let harness = SimpleTree::initialize();


    let copy_result = fs_more::file::copy_file(
        harness.yes.hello_world_txt.as_path(),
        harness.yes.no_bin.as_path(),
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Skip,
        },
    );

    assert_matches!(copy_result.unwrap(), CopyFileFinished::Skipped);


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
pub fn copy_file_errors_when_source_path_is_symlink_to_destination_file() -> TestResult {
    let harness = SimpleTree::initialize();


    let source_symlink_path = harness.child_path("symlink");
    source_symlink_path.assert_not_exists();
    source_symlink_path.symlink_to_file(harness.yes.hello_world_txt.as_path());


    let copy_result = fs_more::file::copy_file(
        &source_symlink_path,
        harness.yes.hello_world_txt.as_path(),
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        FileError::SourceAndDestinationAreTheSame { path }
        if path == harness.yes.hello_world_txt.as_path()
    );


    harness.destroy();
    Ok(())
}



/// **On Windows**, creating symbolic links requires administrator privileges, unless Developer mode is enabled.
/// See [https://stackoverflow.com/questions/58038683/allow-mklink-for-a-non-admin-user].
#[test]
pub fn copy_file_does_not_preserve_symlinks() -> TestResult {
    let harness = SimpleTree::initialize();


    let symlink_path = harness.child_path("symlink");
    symlink_path.assert_not_exists();
    symlink_path.symlink_to_file(harness.yes.no_bin.as_path());


    let symlink_destination_file_size_bytes = harness.yes.no_bin.as_path().size_in_bytes();


    let copy_destination_path = harness.child_path("destination-file");
    copy_destination_path.assert_not_exists();


    let copy_result = fs_more::file::copy_file(
        &symlink_path,
        &copy_destination_path,
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        },
    );


    assert_matches!(
        copy_result.unwrap(),
        CopyFileFinished::Created { bytes_copied }
        if bytes_copied == symlink_destination_file_size_bytes
    );


    symlink_path.assert_is_symlink_to_file_and_destination_matches(harness.yes.no_bin.as_path());
    copy_destination_path.assert_is_file_and_not_symlink();

    harness
        .yes
        .no_bin
        .assert_initial_state_matches_other_file(&copy_destination_path);


    harness.destroy();
    Ok(())
}
