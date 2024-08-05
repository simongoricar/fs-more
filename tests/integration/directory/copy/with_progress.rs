use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        DestinationDirectoryRule,
        DirectoryCopyOperation,
        DirectoryCopyProgress,
        DirectoryCopyWithProgressOptions,
        DirectoryScanDepthLimit,
        DirectoryScanOptions,
        ExistingSubDirectoryBehaviour,
    },
    error::{
        CopyDirectoryError,
        CopyDirectoryPreparationError,
        DestinationDirectoryPathValidationError,
        DirectoryExecutionPlanError,
    },
    file::{ExistingFileBehaviour, FileCopyOptions},
};
use fs_more_test_harness::{
    collect_directory_statistics_via_scan,
    collect_directory_statistics_via_scan_with_options,
    prelude::*,
    trees::structures::{deep::DeepTree, empty::EmptyTree},
};



#[test]
pub fn copy_directory_with_progress_creates_an_identical_copy() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let deep_harness_stats = collect_directory_statistics_via_scan(deep_harness.as_path()).unwrap();


    let mut last_progress_report: Option<DirectoryCopyProgress> = None;

    let finished_copy = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |progress| {
            if let Some(previous_report) = &last_progress_report {
                if previous_report.bytes_total != progress.bytes_total {
                    panic!(
                        "invalid progress reported: bytes_total changed \
                        (got from {} to {})",
                        previous_report.bytes_total,
                        progress.bytes_total,
                    );
                }

                if previous_report.bytes_finished > progress.bytes_finished {
                    panic!(
                        "invalid progress reported: bytes_finished must never decrease \
                        (got from {} to {})",
                        previous_report.bytes_finished,
                        progress.bytes_finished
                    );
                }

                if previous_report.files_copied > progress.files_copied {
                    panic!(
                        "invalid progress reported: files_copied must never decrease \
                        (got from {} to {})",
                        previous_report.files_copied,
                        progress.files_copied
                    );
                }

                if previous_report.directories_created > progress.directories_created {
                    panic!(
                        "invalid progress reported: directories_created must never decrease \
                        (got from {} to {})",
                        previous_report.directories_created,
                        progress.directories_created
                    );
                }

                if previous_report.total_operations != progress.total_operations {
                    panic!(
                        "invalid progress reported: total_operations must never change \
                        (got change from {} to {})",
                        previous_report.total_operations,
                        progress.total_operations,
                    );
                }

                if previous_report.current_operation_index != progress.current_operation_index {
                    if (previous_report.current_operation_index + 1) != progress.current_operation_index {
                        panic!(
                            "invalid progress reported: current_operation_index must always increase by one \
                            (got change from {} to {})",
                            previous_report.current_operation_index,
                            progress.current_operation_index
                        );
                    }
                } else {
                    let previous_path = match &previous_report.current_operation {
                        DirectoryCopyOperation::CreatingDirectory { destination_directory_path } => destination_directory_path.as_path(),
                        DirectoryCopyOperation::CopyingFile { destination_file_path, .. } => destination_file_path.as_path(),
                        DirectoryCopyOperation::CreatingSymbolicLink { destination_symbolic_link_file_path } => destination_symbolic_link_file_path.as_path()
                    };

                    let current_path = match &progress.current_operation {
                        DirectoryCopyOperation::CreatingDirectory { destination_directory_path } => destination_directory_path.as_path(),
                        DirectoryCopyOperation::CopyingFile { destination_file_path, .. } => destination_file_path.as_path(),
                        DirectoryCopyOperation::CreatingSymbolicLink { destination_symbolic_link_file_path } => destination_symbolic_link_file_path.as_path()
                    };

                    if previous_path != current_path {
                        panic!(
                            "invalid progress reported: path in current_operation must not change without \
                            incrementing the current_operation_index \
                            (got change from {} to {})",
                            previous_path.display(),
                            current_path.display(),
                        );
                    }
                }
            }


            last_progress_report = Some(progress.to_owned_progress());
        }
    ).unwrap();


    let last_progress_report = last_progress_report.unwrap();


    assert_eq!(
        last_progress_report.current_operation_index + 1,
        last_progress_report.total_operations,
    );

    assert_eq!(last_progress_report.bytes_total, deep_harness_stats.total_bytes);

    assert_eq!(last_progress_report.bytes_total, last_progress_report.bytes_finished);

    assert_eq!(finished_copy.total_bytes_copied, last_progress_report.bytes_finished);

    assert_eq!(last_progress_report.files_copied, finished_copy.files_copied);

    assert_eq!(
        last_progress_report.directories_created,
        finished_copy.directories_created
    );

    assert_eq!(deep_harness_stats.total_files, finished_copy.files_copied);

    assert_eq!(
        deep_harness_stats.total_directories,
        finished_copy.directories_created
    );


    deep_harness.assert_is_directory_and_fully_matches_secondary_directory(empty_harness.as_path());


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_respects_copy_depth_limit() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    const MAXIMUM_SCAN_DEPTH: DirectoryScanDepthLimit =
        DirectoryScanDepthLimit::Limited { maximum_depth: 2 };

    const MAXIMUM_COPY_DEPTH: CopyDirectoryDepthLimit =
        CopyDirectoryDepthLimit::Limited { maximum_depth: 2 };


    let deep_harness_stats = collect_directory_statistics_via_scan_with_options(
        deep_harness.as_path(),
        DirectoryScanOptions {
            yield_base_directory: false,
            maximum_scan_depth: MAXIMUM_SCAN_DEPTH,
            ..Default::default()
        },
    )
    .unwrap();


    let finished_copy = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: MAXIMUM_COPY_DEPTH,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_copy.total_bytes_copied, deep_harness_stats.total_bytes);


    let destination_harness_stats =
        collect_directory_statistics_via_scan(empty_harness.as_path()).unwrap();


    assert_eq!(deep_harness_stats.total_bytes, destination_harness_stats.total_bytes);


    empty_harness.destroy();
    deep_harness.destroy();
    Ok(())
}




#[test]
pub fn copy_directory_with_progress_errors_when_source_and_destination_are_the_same() -> TestResult
{
    let deep_harness = DeepTree::initialize();


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        deep_harness.as_path(),
        DirectoryCopyWithProgressOptions {
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
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.as_path())
    );


    deep_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_errors_when_destination_is_inside_source_path() -> TestResult {
    let deep_harness = DeepTree::initialize();


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        deep_harness.foo.as_path(),
        DirectoryCopyWithProgressOptions {
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
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.foo.as_path())
    );


    deep_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_errors_when_destination_directory_already_exists_and_rule_is_disallow_existing(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::AlreadyExists { path, destination_directory_rule }
            )
        )
        if paths_equal_no_unc(&path, empty_harness.as_path())
            && destination_directory_rule == DestinationDirectoryRule::DisallowExisting
    );


    deep_harness.assert_is_directory_and_not_empty();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}




#[test]
pub fn copy_directory_with_progress_errors_when_destination_file_collides_and_its_behaviour_is_abort(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // Manually copy one deep harness file over to the empty harness.
    // Afterwards, we'll copy the entire tree over with [`ExistingFileBehaviour::Abort`],
    // meaning the call should error (because there is already a colliding file in the destination).
    let colliding_file_path = {
        empty_harness.assert_is_directory_and_empty();

        let colliding_file_name = deep_harness.a_bin.as_path().file_name().unwrap();
        let empty_harness_colliding_file_path = empty_harness.child_path(colliding_file_name);

        empty_harness_colliding_file_path.assert_not_exists();


        fs_more::file::copy_file(
            deep_harness.foo.bar.c_bin.as_path(),
            &empty_harness_colliding_file_path,
            FileCopyOptions {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            },
        )
        .unwrap();


        empty_harness_colliding_file_path.assert_is_file_and_not_symlink();

        deep_harness
            .foo
            .bar
            .c_bin
            .assert_initial_state_matches_other_file(&empty_harness_colliding_file_path);

        empty_harness.assert_is_directory_and_not_empty();

        empty_harness_colliding_file_path
    };


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
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
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        ) if paths_equal_no_unc(&path, &colliding_file_path)
    );


    colliding_file_path.assert_is_file_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_errors_when_destination_subdirectory_collides_and_its_behaviour_is_abort(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // Manually copy one deep harness directory over to the empty harness.
    // Afterwards, we'll copy the entire tree over with [`ExistingSubDirectoryBehaviour::Abort`],
    // meaning the call should error (because there is already an existing colliding directory in the destination).
    let colliding_directory_path = {
        empty_harness.assert_is_directory_and_empty();


        let colliding_directory_name = deep_harness.foo.as_path().file_name().unwrap();
        let empty_harness_colliding_directory_path =
            empty_harness.child_path(colliding_directory_name);


        empty_harness_colliding_directory_path.assert_not_exists_and_create_empty_directory();
        empty_harness.assert_is_directory_and_not_empty();

        empty_harness_colliding_directory_path
    };


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        )
        if paths_equal_no_unc(&path, &colliding_directory_path)
    );


    colliding_directory_path.assert_is_empty_directory_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_does_not_preserve_file_symlinks() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let (source_symlink_path, remapped_destination_symlink_path) = {
        let source_symlink_path = deep_harness.child_path("symlink");
        source_symlink_path.assert_not_exists();
        source_symlink_path.symlink_to_file(deep_harness.foo.bar.c_bin.as_path());


        let remapped_destination_symlink_path = empty_harness.child_path("symlink");
        remapped_destination_symlink_path.assert_not_exists();


        (source_symlink_path, remapped_destination_symlink_path)
    };



    fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    source_symlink_path.assert_is_symlink_to_file();

    remapped_destination_symlink_path.assert_is_file_and_not_symlink();
    deep_harness
        .foo
        .bar
        .c_bin
        .assert_initial_state_matches_other_file(&remapped_destination_symlink_path);


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_does_not_preserve_directory_symlinks() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let (source_symlink_path, remapped_destination_symlink_path) = {
        let source_symlink_path = deep_harness.child_path("dir-symlink");
        source_symlink_path.assert_not_exists();
        source_symlink_path.symlink_to_directory(deep_harness.foo.bar.as_path());


        let remapped_destination_symlink_path = empty_harness.child_path("dir-symlink");
        remapped_destination_symlink_path.assert_not_exists();


        (source_symlink_path, remapped_destination_symlink_path)
    };

    source_symlink_path
        .assert_is_symlink_to_directory_and_resolve_destination()
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.foo.bar.as_path());


    fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();



    remapped_destination_symlink_path.assert_is_directory_and_not_symlink();
    remapped_destination_symlink_path
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.foo.bar.as_path());



    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_respects_copy_depth_limit_even_if_source_contains_symlink(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let (
        remapped_destination_symlink_path,
        remapped_c_bin_path_inside_symlink,
        remapped_hello_dir_path_inside_symlink,
    ) = {
        let source_symlink_path = deep_harness.child_path("dir-symlink");
        source_symlink_path.assert_not_exists();
        source_symlink_path.symlink_to_directory(deep_harness.foo.bar.as_path());


        let remapped_destination_symlink_path = empty_harness.child_path("dir-symlink");

        let remapped_c_bin_path_inside_symlink = remapped_destination_symlink_path
            .join(deep_harness.foo.bar.c_bin.as_path().file_name().unwrap());

        let remapped_hello_dir_path_inside_symlink = remapped_destination_symlink_path
            .join(deep_harness.foo.bar.hello.as_path().file_name().unwrap());


        remapped_destination_symlink_path.assert_not_exists();
        remapped_c_bin_path_inside_symlink.assert_not_exists();

        (
            remapped_destination_symlink_path,
            remapped_c_bin_path_inside_symlink,
            remapped_hello_dir_path_inside_symlink,
        )
    };


    fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            copy_depth_limit: CopyDirectoryDepthLimit::Limited { maximum_depth: 1 },
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    remapped_destination_symlink_path.assert_is_directory_and_not_symlink();
    remapped_destination_symlink_path.assert_is_directory_and_not_empty();
    remapped_c_bin_path_inside_symlink.assert_is_file_and_not_symlink();

    deep_harness
        .foo
        .bar
        .c_bin
        .assert_initial_state_matches_other_file(&remapped_c_bin_path_inside_symlink);

    remapped_hello_dir_path_inside_symlink.assert_is_directory_and_not_symlink();
    remapped_hello_dir_path_inside_symlink.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_preemptively_checks_for_directory_collisions() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let remapped_colliding_directory_path = {
        empty_harness.assert_is_directory_and_empty();

        let relative_path_to_bar = deep_harness.foo.as_path_relative_to_harness_root();

        let remapped_path = empty_harness.child_path(relative_path_to_bar);
        remapped_path.assert_not_exists_and_create_empty_directory();

        empty_harness.assert_is_directory_and_not_empty();

        remapped_path
    };


    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        )
        if paths_equal_no_unc(&path, &remapped_colliding_directory_path)
    );


    remapped_colliding_directory_path.assert_is_empty_directory_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn copy_directory_with_progress_preemptively_checks_for_file_collisions() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let remapped_colliding_file_path = {
        empty_harness.assert_is_directory_and_empty();

        let relative_path_to_a_bin = deep_harness.a_bin.as_path_relative_to_harness_root();

        let remapped_path = empty_harness.child_path(relative_path_to_a_bin);

        fs_more::file::copy_file(
            deep_harness.a_bin.as_path(),
            &remapped_path,
            FileCopyOptions {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            },
        )
        .unwrap();

        empty_harness.assert_is_directory_and_not_empty();

        remapped_path
    };


    let mut last_progress_report: Option<DirectoryCopyProgress> = None;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |progress| {
            last_progress_report = Some(progress.to_owned_progress());
        },
    );


    assert!(last_progress_report.is_none());

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        )
        if paths_equal_no_unc(&path, &remapped_colliding_file_path)
    );



    remapped_colliding_file_path.assert_is_file_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
    Ok(())
}


/// Tests fs_more behaviour when copying a "symlink to directory A" to "A".
/// This should return an error, regardless of overwriting configuration.
#[test]
pub fn copy_directory_with_progress_errors_when_source_is_symlink_to_destination() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());


    let symlink_to_deep_harnesss_path = {
        let symlink_path = empty_harness.child_path("directory-symlink");
        symlink_path.assert_not_exists();

        symlink_path.symlink_to_directory(deep_harness.as_path());

        empty_harness.assert_is_directory_and_not_empty();

        symlink_path
    };


    let mut last_progress_report: Option<DirectoryCopyProgress> = None;

    let copy_result = fs_more::directory::copy_directory_with_progress(
        symlink_to_deep_harnesss_path.as_path(),
        deep_harness.as_path(),
        DirectoryCopyWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
        |progress| {
            last_progress_report = Some(progress.to_owned_progress());
        },
    );


    assert!(last_progress_report.is_none());

    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.as_path())
    );


    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());


    symlink_to_deep_harnesss_path.assert_is_symlink_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    deep_harness_untouched.destroy();
    empty_harness.destroy();
    Ok(())
}


// TODO Revisit tests that handle symlinks: remove obsolete tests and add new ones that test the symlink options.
