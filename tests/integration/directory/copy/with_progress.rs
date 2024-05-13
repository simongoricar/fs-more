use assert_matches::assert_matches;
use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        CopyDirectoryOptions,
        CopyDirectoryProgress,
        CopyDirectoryWithProgressOptions,
        DestinationDirectoryRule,
        DirectoryScan,
        DirectoryScanDepthLimit,
        ExistingSubDirectoryBehaviour,
    },
    error::{
        CopyDirectoryError,
        CopyDirectoryPlanError,
        CopyDirectoryPreparationError,
        DestinationDirectoryPathValidationError,
    },
    file::{CopyFileOptions, ExistingFileBehaviour},
};
use fs_more_test_harness::{
    assertable::{AssertableDirectoryPath, AssertableFilePath},
    error::TestResult,
    trees::{DeepTreeHarness, EmptyTreeHarness},
};



#[test]
pub fn copy_directory_with_progress() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(
        harness.root.path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let mut last_progress: Option<CopyDirectoryProgress> = None;

    let finished_copy = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
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
        source_scan.files().len(),
        last_progress.files_copied,
        "copy_directory_with_progress's last progress did not report all files"
    );
    assert_eq!(
        source_scan.files().len(),
        finished_copy.files_copied,
        "DirectoryScan and copy_directory_with_progress report different number of files"
    );

    assert_eq!(
        source_scan.directories().len(),
        last_progress.directories_created,
        "copy_directory_with_progress's last progress did not report all directories"
    );
    assert_eq!(
        source_scan.directories().len(),
        finished_copy.directories_created,
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
pub fn copy_directory_with_progress_respects_depth_option() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    const MAXIMUM_DEPTH: usize = 2;

    let source_scan = DirectoryScan::scan_with_options(
        harness.root.path(),
        DirectoryScanDepthLimit::Limited {
            maximum_depth: MAXIMUM_DEPTH,
        },
        false,
    )
    .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: CopyDirectoryDepthLimit::Limited {
                maximum_depth: MAXIMUM_DEPTH,
            },
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

    let target_scan = DirectoryScan::scan_with_options(
        empty_harness.root.path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
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
pub fn disallow_copy_directory_with_progress_on_existing_file_without_option() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    // Still the harness setup.
    let file_a_filename = harness.file_a.path().file_name().unwrap();
    let test_file_path = empty_harness.root.child_path(file_a_filename);
    fs_more::file::copy_file(
        harness.file_a.path(),
        &test_file_path,
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
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
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |_| {},
    );

    assert!(
        copy_result.is_err(),
        "copy_directory_with_progress should have errored due to existing destination file"
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists {
                    path
                }
            )
        ) if path == test_file.path()
    );


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_with_progress_on_existing_directory_without_option() -> TestResult {
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
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    );

    assert!(
        copy_result.is_err(),
        "copy_directory_with_progress should have errored due to existing destination file"
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        )
        if path == replicated_foo_dir_path
    );


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_with_progress_into_itself() -> TestResult {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        ) if source_directory_path == harness.root.path() && destination_directory_path == harness.root.path()
    );

    harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_with_progress_into_subdirectory_of_itself() -> TestResult {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        harness.dir_world.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        ) if source_directory_path == harness.root.path() && destination_directory_path == harness.dir_world.path()
    );

    harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_does_not_preserve_file_symlinks() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    let symlinked_file =
        AssertableFilePath::from_path(harness.root.child_path("file_a-symlinked.bin"));

    symlinked_file.assert_not_exists();
    symlinked_file.symlink_to_file(harness.file_a.path())?;
    symlinked_file.assert_is_symlink_to_file();


    fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();


    let previously_symlinked_file_in_target =
        AssertableFilePath::from_path(empty_harness.root.child_path("file_a-symlinked.bin"));

    previously_symlinked_file_in_target.assert_exists();
    previously_symlinked_file_in_target.assert_is_file();


    empty_harness.destroy()?;
    harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_does_not_preserve_directory_symlinks() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    let symlinked_dir =
        AssertableDirectoryPath::from_path(harness.root.child_path("symlinked-directory"));

    symlinked_dir.assert_not_exists();
    symlinked_dir.symlink_to_directory(harness.dir_foo.path())?;
    symlinked_dir.assert_is_symlink_to_directory();


    fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions::default(),
    )
    .unwrap();


    let previously_symlinked_dir_in_target =
        AssertableDirectoryPath::from_path(empty_harness.root.child_path("symlinked-directory"));

    previously_symlinked_dir_in_target.assert_exists();
    previously_symlinked_dir_in_target.assert_is_directory();
    previously_symlinked_dir_in_target
        .assert_directory_contents_match_directory(harness.dir_foo.path());


    empty_harness.destroy()?;
    harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_does_not_preserve_directory_symlink() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    let symlinked_dir =
        AssertableDirectoryPath::from_path(harness.root.child_path("symlinked-directory"));

    symlinked_dir.assert_not_exists();
    symlinked_dir.symlink_to_directory(harness.dir_foo.path())?;
    symlinked_dir.assert_is_symlink_to_directory();


    fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();


    let previously_symlinked_dir_in_target =
        AssertableDirectoryPath::from_path(empty_harness.root.child_path("symlinked-directory"));

    previously_symlinked_dir_in_target.assert_exists();
    previously_symlinked_dir_in_target.assert_is_directory();
    previously_symlinked_dir_in_target
        .assert_directory_contents_match_directory(harness.dir_foo.path());


    empty_harness.destroy()?;
    harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_containing_symbolic_link_to_directory_and_respects_depth_limit(
) -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    let symlinked_dir =
        AssertableDirectoryPath::from_path(harness.root.child_path("symlinked-directory"));

    symlinked_dir.assert_not_exists();
    symlinked_dir.symlink_to_directory(harness.dir_foo.path())?;
    symlinked_dir.assert_is_symlink_to_directory();


    fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions {
            copy_depth_limit: CopyDirectoryDepthLimit::Limited { maximum_depth: 1 },
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    let previously_symlinked_dir_in_target =
        AssertableDirectoryPath::from_path(empty_harness.root.child_path("symlinked-directory"));

    previously_symlinked_dir_in_target.assert_exists();
    previously_symlinked_dir_in_target.assert_is_directory();


    let previously_symlinked_file_b = AssertableFilePath::from_path(
        previously_symlinked_dir_in_target
            .path()
            .join(harness.file_b.path().file_name().unwrap()),
    );

    previously_symlinked_file_b.assert_is_file();
    previously_symlinked_file_b.assert_content_matches_file(harness.file_b.path());


    let previously_symlinked_file_c = AssertableFilePath::from_path(
        previously_symlinked_dir_in_target
            .path()
            .join(harness.dir_bar.path().file_name().unwrap())
            .join(harness.file_c.path().file_name().unwrap()),
    );

    previously_symlinked_file_c.assert_not_exists();


    empty_harness.destroy()?;
    harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_preemptively_checks_for_directory_collisions() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;
    empty_harness.root.assert_is_empty();

    // Target directory preparation.
    let existing_target_file = AssertableFilePath::from_path(
        empty_harness.root.path().join(
            harness
                .file_d
                .path()
                .strip_prefix(harness.root.path())
                .unwrap(),
        ),
    );

    let non_existing_target_file = AssertableFilePath::from_path(
        empty_harness.root.path().join(
            harness
                .file_a
                .path()
                .strip_prefix(harness.root.path())
                .unwrap(),
        ),
    );

    std::fs::create_dir_all(existing_target_file.path().parent().unwrap()).unwrap();
    std::fs::copy(harness.file_d.path(), existing_target_file.path()).unwrap();

    existing_target_file.assert_content_matches_file(harness.file_d.path());
    non_existing_target_file.assert_not_exists();

    empty_harness.root.assert_is_not_empty();
    // END of preparation

    let mut last_progress: Option<CopyDirectoryProgress> = None;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |progress| {
            last_progress = Some(progress.clone());
        },
    );

    assert!(
        last_progress.is_none(),
        "copy_directory_with_progress did not check for directory collisions before starting copy"
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == empty_harness.root.path()
                        .join(
                            harness.dir_foo.path()
                                .strip_prefix(harness.root.path())
                                .unwrap()
                        )
    );


    empty_harness.root.assert_is_not_empty();

    non_existing_target_file.assert_not_exists();
    existing_target_file.assert_content_matches_file(harness.file_d.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_preemptively_checks_for_file_collisions() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;
    empty_harness.root.assert_is_empty();

    // Target directory preparation.
    let existing_target_file = AssertableFilePath::from_path(
        empty_harness.root.path().join(
            harness
                .file_d
                .path()
                .strip_prefix(harness.root.path())
                .unwrap(),
        ),
    );

    let non_existing_target_file = AssertableFilePath::from_path(
        empty_harness.root.path().join(
            harness
                .file_a
                .path()
                .strip_prefix(harness.root.path())
                .unwrap(),
        ),
    );

    std::fs::create_dir_all(existing_target_file.path().parent().unwrap()).unwrap();
    std::fs::copy(harness.file_d.path(), existing_target_file.path()).unwrap();

    existing_target_file.assert_content_matches_file(harness.file_d.path());
    non_existing_target_file.assert_not_exists();

    empty_harness.root.assert_is_not_empty();
    // END of preparation

    let mut last_progress: Option<CopyDirectoryProgress> = None;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |progress| {
            last_progress = Some(progress.clone());
        },
    );


    assert!(
        last_progress.is_none(),
        "copy_directory_with_progress did not check for directory collisions before starting copy"
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == existing_target_file.path()
    );

    empty_harness.root.assert_is_not_empty();

    non_existing_target_file.assert_not_exists();
    existing_target_file.assert_content_matches_file(harness.file_d.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_with_progress_when_source_is_symlink_to_target() -> TestResult {
    // Tests behaviour when copying "symlink to directory A" to "A".
    // This should fail.

    let harness_for_comparison = DeepTreeHarness::new()?;
    let harness = DeepTreeHarness::new()?;
    let intermediate_harness = EmptyTreeHarness::new()?;

    // Directory symlink preparation
    let symlink_to_directory = AssertableDirectoryPath::from_path(
        intermediate_harness.root.child_path("symlinked-directory"),
    );

    symlink_to_directory.assert_not_exists();
    symlink_to_directory
        .symlink_to_directory(harness.root.path())
        .unwrap();
    // END of preparation

    let mut last_progress: Option<CopyDirectoryProgress> = None;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        symlink_to_directory.path(),
        harness.root.path(),
        CopyDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |progress| {
            last_progress = Some(progress.clone());
        },
    );

    assert!(last_progress.is_none());

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        ) if source_directory_path == harness.root.path() && destination_directory_path == harness.root.path()
    );

    harness
        .root
        .assert_directory_contents_match_directory(harness_for_comparison.root.path());


    harness_for_comparison.destroy()?;
    harness.destroy()?;
    intermediate_harness.destroy()?;

    Ok(())
}