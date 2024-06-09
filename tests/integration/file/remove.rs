use assert_matches::assert_matches;
use fs_more::error::FileRemoveError;
use fs_more_test_harness::{
    assertable::{
        r#trait::{AssertablePath, CaptureableFilePath, ManageablePath},
        AsPath,
    },
    error::TestResult,
    tree_framework::{FileSystemHarness, FileSystemHarnessDirectory},
    trees::{empty::EmptyTree, simple::SimpleTree},
};




#[test]
pub fn remove_file_deletes_file() -> TestResult {
    let harness = SimpleTree::initialize();

    let removal_result = fs_more::file::remove_file(harness.foo.hello_world_txt.as_path());


    assert!(
        removal_result.is_ok(),
        "failed to remove file: expected Ok, got {}",
        removal_result.unwrap_err()
    );

    harness.empty_txt.assert_exists();
    harness.foo.hello_world_txt.assert_not_exists();

    harness.destroy();
    Ok(())
}



#[test]
pub fn remove_file_does_not_follow_symlinks() -> TestResult {
    let harness = SimpleTree::initialize();
    let secondary_harness = SimpleTree::initialize();

    harness.empty_txt.assert_is_file_and_remove();
    harness.empty_txt.assert_not_exists();

    secondary_harness.empty_txt.assert_is_file_and_not_symlink();
    harness
        .empty_txt
        .symlink_to_file(secondary_harness.empty_txt.as_path());
    harness
        .empty_txt
        .assert_is_symlink_to_file_and_destination_matches(secondary_harness.empty_txt.as_path());

    let captured_symlink_destination_file = secondary_harness.empty_txt.capture_with_content();


    fs_more::file::remove_file(harness.empty_txt.as_path()).unwrap();


    harness.empty_txt.assert_not_exists();
    captured_symlink_destination_file.assert_unchanged();


    harness.destroy();
    Ok(())
}



#[test]
pub fn remove_file_errors_on_non_existent_file() -> TestResult {
    let harness = EmptyTree::initialize();

    let non_existent_file = harness.child_path("hello-world.txt");
    non_existent_file.assert_not_exists();


    let removal_result = fs_more::file::remove_file(&non_existent_file);


    assert!(
        removal_result.is_err(),
        "failed to error on file removal: expected Err, got Ok"
    );

    assert_matches!(
        removal_result.unwrap_err(),
        FileRemoveError::NotFound { path }
        if path == non_existent_file
    );


    harness.destroy();
    Ok(())
}
