use assert_matches::assert_matches;
use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        CopyDirectoryOptions,
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
pub fn copy_directory() -> TestResult {
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

    let finished_copy = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
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
        source_scan.files().len(),
        finished_copy.files_copied,
        "DirectoryScan and copy_directory report different number of files"
    );

    assert_eq!(
        source_scan.directories().len(),
        finished_copy.directories_created,
        "DirectoryScan and copy_directory report different number of directories"
    );

    harness
        .root
        .assert_directory_contents_fully_match_directory(empty_harness.root.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_respects_maximum_depth_option() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    const MAXIMUM_SCAN_DEPTH: DirectoryScanDepthLimit =
        DirectoryScanDepthLimit::Limited { maximum_depth: 2 };

    const MAXIMUM_COPY_DEPTH: CopyDirectoryDepthLimit =
        CopyDirectoryDepthLimit::Limited { maximum_depth: 2 };


    let source_scan =
        DirectoryScan::scan_with_options(harness.root.path(), MAXIMUM_SCAN_DEPTH, false)
            .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let finished_copy = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: MAXIMUM_COPY_DEPTH,
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
        source_scan.files().len(),
        finished_copy.files_copied,
        "DirectoryScan and copy_directory report different number of files"
    );

    assert_eq!(
        source_scan.directories().len(),
        finished_copy.directories_created,
        "DirectoryScan and copy_directory report different number of directories"
    );

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_into_itself() -> TestResult {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
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
pub fn disallow_copy_directory_into_subdirectory_of_itself() -> TestResult {
    let harness = DeepTreeHarness::new()?;

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        harness.dir_world.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
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
pub fn disallow_copy_directory_to_existing_destination_directory_without_option() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    empty_harness.root.assert_is_empty();

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
            ..Default::default()
        },
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::AlreadyExists { path, .. }
            )
        ) if path == empty_harness.root.path()
    );

    empty_harness.root.assert_is_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_to_non_empty_destination_without_option() -> TestResult {
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

    let test_file = AssertableFilePath::from_path_with_captured_content(&test_file_path)?;

    test_file.assert_exists();
    test_file.assert_content_unchanged();
    // End of setup, we have now pre-copied a single file to test our overwriting options.


    empty_harness.root.assert_is_not_empty();

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == test_file_path
    );

    empty_harness.root.assert_is_not_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_to_non_empty_destination_with_subdirectory_without_option(
) -> TestResult {
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
        CopyFileOptions {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
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
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == replicated_foo_dir_path
    );

    empty_harness.root.assert_is_not_empty();

    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_does_not_preserve_file_symlinks() -> TestResult {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;


    let symlinked_file =
        AssertableFilePath::from_path(harness.root.child_path("file_a-symlinked.bin"));

    symlinked_file.assert_not_exists();
    symlinked_file.symlink_to_file(harness.file_a.path())?;
    symlinked_file.assert_is_symlink_to_file();


    fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions::default(),
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
pub fn copy_directory_containing_symbolic_link_to_directory_and_respects_depth_limit() -> TestResult
{
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
        CopyDirectoryOptions {
            copy_depth_limit: CopyDirectoryDepthLimit::Limited { maximum_depth: 1 },
            ..Default::default()
        },
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
pub fn copy_directory_preemptively_checks_for_directory_collisions() -> TestResult {
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

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == empty_harness.root.path().join(harness.dir_foo.path().strip_prefix(harness.root.path()).unwrap())
    );

    empty_harness.root.assert_is_not_empty();

    non_existing_target_file.assert_not_exists();
    existing_target_file.assert_content_matches_file(harness.file_d.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn copy_directory_preemptively_checks_for_file_collisions() -> TestResult {
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

    let copy_result = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );


    let expected_errored_path = empty_harness.root.path().join(
        harness
            .dir_foo
            .path()
            .strip_prefix(harness.root.path())
            .unwrap(),
    );

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                CopyDirectoryPlanError::DestinationItemAlreadyExists { path }
            )
        ) if path == expected_errored_path,
        "DestinationItemAlreadyExists {{ path }} does not match path {}",
        expected_errored_path.display()
    );

    empty_harness.root.assert_is_not_empty();

    non_existing_target_file.assert_not_exists();
    existing_target_file.assert_content_matches_file(harness.file_d.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}



#[test]
pub fn disallow_copy_directory_when_source_is_symlink_to_destination() -> TestResult {
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

    let copy_result = fs_more::directory::copy_directory(
        symlink_to_directory.path(),
        harness.root.path(),
        CopyDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );

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
        .assert_directory_contents_fully_match_directory(harness_for_comparison.root.path());


    harness_for_comparison.destroy()?;
    harness.destroy()?;
    intermediate_harness.destroy()?;

    Ok(())
}
