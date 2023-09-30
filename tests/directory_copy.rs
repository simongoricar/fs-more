use assert_matches::assert_matches;
use fs_more::{
    directory::{
        DirectoryCopyOptions,
        DirectoryCopyProgress,
        DirectoryCopyWithProgressOptions,
        DirectoryScan,
    },
    error::DirectoryError,
    file::FileCopyOptions,
};
use fs_more_test_harness::{
    assertable::AssertableFilePath,
    error::TestResult,
    trees::{DeepTreeHarness, EmptyTreeHarness},
};

#[test]
pub fn copy_directory() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), None, false)
        .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let finished_copy = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory unexpectedly failed with Err: {}",
            error
        );
    });


    assert_eq!(
        source_full_size, finished_copy.total_bytes_copied,
        "DirectoryScan and copy_directory report different amount of bytes"
    );

    assert_eq!(
        source_scan.files.len(),
        finished_copy.num_files_copied,
        "DirectoryScan and copy_directory report different number of files"
    );

    assert_eq!(
        source_scan.directories.len(),
        finished_copy.num_directories_created,
        "DirectoryScan and copy_directory report different number of directories"
    );

    harness
        .root
        .assert_directory_contents_match_directory(empty_harness.root.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}


#[test]
pub fn copy_directory_respect_maximum_depth_option() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    const MAXIMUM_DEPTH: Option<usize> = Some(2);

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), MAXIMUM_DEPTH, false)
        .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let finished_copy = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            maximum_copy_depth: MAXIMUM_DEPTH,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory unexpectedly failed with Err: {}",
            error
        );
    });

    assert_eq!(
        source_full_size, finished_copy.total_bytes_copied,
        "DirectoryScan and copy_directory report different amount of bytes"
    );

    assert_eq!(
        source_scan.files.len(),
        finished_copy.num_files_copied,
        "DirectoryScan and copy_directory report different number of files"
    );

    assert_eq!(
        source_scan.directories.len(),
        finished_copy.num_directories_created,
        "DirectoryScan and copy_directory report different number of directories"
    );

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), None, false)
        .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let mut last_progress: Option<DirectoryCopyProgress> = None;

    let finished_copy = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
        |progress| {
            if last_progress.is_none() {
                last_progress = Some(progress.clone());
                return;
            };

            let previous_progress = last_progress.as_ref().unwrap();
            let progress_operation_index_delta = progress.current_operation_index - previous_progress.current_operation_index;

            if progress_operation_index_delta != 0 && progress_operation_index_delta != 1 {
                panic!(
                    "copy_directory_with_progress reported non-consecutive operation indexes: {} and {}",
                    previous_progress.current_operation_index,
                    progress.current_operation_index
                );
            }

            assert!(
                progress.current_operation_index >= 0,
                "copy_directory_with_progress reported a negative operation index: {}",
                progress.current_operation_index
            );

            last_progress = Some(progress.clone());
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory_with_progress unexpectedly failed with Err: {}",
            error
        );
    });


    assert!(
        last_progress.is_some(),
        "copy_directory_with_progress did not report progress at all"
    );

    let last_progress = last_progress.unwrap();

    assert_eq!(
        last_progress.current_operation_index + 1,
        last_progress.total_operations,
        "copy_directory_with_progress's last progress reported inconsistent operation indexes"
    );

    assert_eq!(
        last_progress.bytes_finished, last_progress.bytes_total,
        "copy_directory_with_progress's last progress message was an unfinished copy"
    );
    assert_eq!(
        source_full_size,
        last_progress.bytes_total,
        "DirectoryScan and copy_directory_with_progress's last progress reported different amount of total bytes"
    );
    assert_eq!(
        source_full_size, finished_copy.total_bytes_copied,
        "DirectoryScan and copy_directory_with_progress report different amount of total bytes"
    );

    assert_eq!(
        source_scan.files.len(),
        last_progress.files_copied,
        "copy_directory_with_progress's last progress did not report all files"
    );
    assert_eq!(
        source_scan.files.len(),
        finished_copy.num_files_copied,
        "DirectoryScan and copy_directory_with_progress report different number of files"
    );

    assert_eq!(
        source_scan.directories.len(),
        last_progress.directories_created,
        "copy_directory_with_progress's last progress did not report all directories"
    );
    assert_eq!(
        source_scan.directories.len(),
        finished_copy.num_directories_created,
        "DirectoryScan and copy_directory_with_progress report different number of directories"
    );

    harness
        .root
        .assert_directory_contents_match_directory(empty_harness.root.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}


#[test]
pub fn copy_directory_with_progress_respect_depth_option() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    const MAXIMUM_DEPTH: Option<usize> = Some(2);

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), MAXIMUM_DEPTH, false)
        .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            maximum_copy_depth: MAXIMUM_DEPTH,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory_with_progress unexpectedly failed with Err: {}",
            error
        );
    });

    let target_scan = DirectoryScan::scan_with_options(empty_harness.root.path(), None, false)
        .expect("failed to scan target temporary directory");
    let target_full_size = target_scan
        .total_size_in_bytes()
        .expect("failed to compute size of target directory in bytes");

    assert_eq!(
        source_full_size, target_full_size,
        "copy_directory_with_progress did not create an equally-sized directory copy"
    );

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}


#[test]
pub fn error_on_copy_directory_with_progress_on_existing_file_without_option() -> TestResult<()> {
    // TODO
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    // Still the harness setup.
    let file_a_filename = harness.file_a.path().file_name().unwrap();
    let test_file_path = empty_harness.root.child_path(file_a_filename);
    fs_more::file::copy_file(
        harness.file_a.path(),
        &test_file_path,
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    )
    .unwrap();

    let test_file = AssertableFilePath::from_path_with_captured_content(test_file_path)?;

    test_file.assert_exists();
    test_file.assert_content_unchanged();
    // End of setup, we have now pre-copied a single file to test our overwriting options.


    empty_harness.root.assert_is_not_empty();

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            overwrite_existing_files: false,
            ..Default::default()
        },
        |_| {},
    );

    assert!(
        copy_result.is_err(),
        "copy_directory_with_progress should have errored due to existing target file"
    );

    let copy_err = copy_result.unwrap_err();
    match &copy_err {
        DirectoryError::TargetItemAlreadyExists { path } => {
            assert_eq!(
                path,
                test_file.path(),
                "copy_directory_with_progress returned TargetItemAlreadyExists with incorrect inner path"
            );
        }
        _ => {
            panic!("copy_directory_with_progress should have errored with TargetItemAlreadyExists")
        }
    }


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}

#[test]
pub fn error_on_copy_directory_with_progress_on_existing_directory_without_option() -> TestResult<()>
{
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    // Still the harness setup.
    let replicated_foo_dir_name = harness.dir_foo.path().file_name().unwrap();
    let replicated_foo_dir_path = empty_harness.root.child_path(replicated_foo_dir_name);
    std::fs::create_dir_all(&replicated_foo_dir_path)?;
    // End of setup, we have now pre-copied a single file to test our overwriting options.


    empty_harness.root.assert_is_not_empty();

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            overwrite_existing_subdirectories: false,
            ..Default::default()
        },
        |_| {},
    );

    assert!(
        copy_result.is_err(),
        "copy_directory_with_progress should have errored due to existing target file"
    );

    let copy_err = copy_result.unwrap_err();
    match &copy_err {
        DirectoryError::TargetItemAlreadyExists { path } => {
            assert_eq!(
                path,
                &replicated_foo_dir_path,
                "copy_directory_with_progress returned TargetItemAlreadyExists with incorrect inner path"
            );
        }
        _ => {
            panic!("copy_directory_with_progress should have errored with TargetItemAlreadyExists")
        }
    }


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_into_itself() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
    );

    assert_matches!(
        copy_result,
        Err(DirectoryError::InvalidTargetDirectoryPath),
        "copy_directory should have errored when trying to copy a directory into itself"
    );

    harness.destroy()?;
    Ok(())
}

#[test]
pub fn disallow_copy_directory_into_subdirectory_of_itself() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        harness.dir_world.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
    );

    assert_matches!(
        copy_result,
        Err(DirectoryError::InvalidTargetDirectoryPath),
        "copy_directory should have errored when trying to \
        copy a directory into a subdirectory of itself"
    );

    harness.destroy()?;
    Ok(())
}

#[test]
pub fn disallow_copy_directory_with_progress_into_itself() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(
        copy_result,
        Err(DirectoryError::InvalidTargetDirectoryPath),
        "copy_directory_with_progress should have errored when trying to \
        copy a directory into itself"
    );

    harness.destroy()?;
    Ok(())
}

#[test]
pub fn disallow_copy_directory_with_progress_into_subdirectory_of_itself() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        harness.dir_world.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(
        copy_result,
        Err(DirectoryError::InvalidTargetDirectoryPath),
        "copy_directory_with_progress should have errored when trying to \
        copy a directory into a subdirectory of itself"
    );

    harness.destroy()?;
    Ok(())
}

#[test]
pub fn error_on_copy_directory_on_existing_target_directory_without_option() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    empty_harness.root.assert_is_empty();

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: false,
            ..Default::default()
        },
    );

    let copy_err = copy_result.unwrap_err();
    match &copy_err {
        DirectoryError::TargetItemAlreadyExists { path } => {
            assert_eq!(
                path,
                empty_harness.root.path(),
                "copy_directory did not return the correct path \
                inside the TargetItemAlreadyExists error"
            );
        }
        _ => panic!("Unexpected Err value: {}", copy_err),
    }

    empty_harness.root.assert_is_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}

#[test]
pub fn error_on_copy_directory_on_existing_file_without_option() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    // Still the harness setup.
    let file_a_filename = harness.file_a.path().file_name().unwrap();
    let test_file_path = empty_harness.root.child_path(file_a_filename);
    fs_more::file::copy_file(
        harness.file_a.path(),
        &test_file_path,
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    )
    .unwrap();

    let test_file = AssertableFilePath::from_path_with_captured_content(test_file_path)?;

    test_file.assert_exists();
    test_file.assert_content_unchanged();
    // End of setup, we have now pre-copied a single file to test our overwriting options.


    empty_harness.root.assert_is_not_empty();

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            overwrite_existing_files: false,
            ..Default::default()
        },
    );

    let copy_err = copy_result.unwrap_err();
    match &copy_err {
        DirectoryError::TargetItemAlreadyExists { path } => {
            assert_eq!(
                path,
                test_file.path(),
                "copy_directory did not return the correct path \
                inside the TargetItemAlreadyExists error"
            );
        }
        _ => panic!("Unexpected Err value: {}", copy_err),
    }

    empty_harness.root.assert_is_not_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}

#[test]
pub fn error_on_copy_directory_on_existing_subdirectory_without_option() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    // Still the harness setup.
    let replicated_foo_dir_name = harness.dir_foo.path().file_name().unwrap();
    let replicated_foo_dir_path = empty_harness.root.child_path(replicated_foo_dir_name);
    std::fs::create_dir_all(&replicated_foo_dir_path)?;

    let file_b_filename = harness.file_b.path().file_name().unwrap();
    let replicated_file_b_path = empty_harness.root.child_path(file_b_filename);
    fs_more::file::copy_file(
        harness.file_b.path(),
        &replicated_file_b_path,
        FileCopyOptions {
            overwrite_existing: false,
            skip_existing: false,
        },
    )
    .unwrap();

    let replicated_file_b =
        AssertableFilePath::from_path_with_captured_content(replicated_file_b_path)?;

    replicated_file_b.assert_exists();
    replicated_file_b.assert_content_unchanged();
    // End of setup, we have now pre-copied a single directory containing
    // a single file to test our overwriting options.


    empty_harness.root.assert_is_not_empty();

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            overwrite_existing_files: true,
            overwrite_existing_subdirectories: false,
            ..Default::default()
        },
    );

    let copy_err = copy_result.unwrap_err();
    match &copy_err {
        DirectoryError::TargetItemAlreadyExists { path } => {
            assert_eq!(
                path, &replicated_foo_dir_path,
                "copy_directory did not return the correct path \
                inside the TargetItemAlreadyExists error"
            );
        }
        _ => panic!("Unexpected Err value: {}", copy_err),
    }

    empty_harness.root.assert_is_not_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}
