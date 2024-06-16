use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        DestinationDirectoryRule,
        DirectoryCopyOptions,
        DirectoryMoveOperation,
        DirectoryMoveProgress,
        DirectoryMoveStrategy,
        DirectoryMoveWithProgressOptions,
        DirectoryScanOptions,
        ExistingSubDirectoryBehaviour,
    },
    error::{
        DestinationDirectoryPathValidationError,
        MoveDirectoryError,
        MoveDirectoryPreparationError,
    },
    file::ExistingFileBehaviour,
};
use fs_more_test_harness::{
    assert_matches,
    assertable::{
        r#trait::{AssertablePath, ManageablePath},
        AsPath,
    },
    error::TestResult,
    paths_equal_no_unc,
    tree_framework::{FileSystemHarness, FileSystemHarnessDirectory},
    trees::{deep::DeepTree, empty::EmptyTree, simple::SimpleTree},
};



#[test]
pub fn move_directory_with_progress_moves_all_files_and_subdirectories() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let source_scan = fs_more::directory::DirectoryScan::scan_with_options(
        deep_harness.as_path(),
        DirectoryScanOptions::default(),
    )
    .unwrap();

    let source_scan_bytes = source_scan.total_size_in_bytes().unwrap();


    let mut last_progress_report: Option<DirectoryMoveProgress> = None;

    let finished_move = fs_more::directory::move_directory_with_progress(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |progress| {
            if let Some(previous_report) = &last_progress_report {
                if previous_report.bytes_total != progress.bytes_total {
                    panic!(
                        "invalid progress reported: bytes_total changed \
                        (got from {} to {})",
                        previous_report.bytes_total, progress.bytes_total,
                    );
                }

                if previous_report.bytes_finished > progress.bytes_finished {
                    panic!(
                        "invalid progress reported: bytes_finished must never decrease \
                        (got from {} to {})",
                        previous_report.bytes_finished, progress.bytes_finished
                    );
                }

                if previous_report.files_moved > progress.files_moved {
                    panic!(
                        "invalid progress reported: files_moved must never decrease \
                        (got from {} to {})",
                        previous_report.files_moved, progress.files_moved
                    );
                }

                if previous_report.directories_created > progress.directories_created {
                    panic!(
                        "invalid progress reported: directories_created must never decrease \
                        (got from {} to {})",
                        previous_report.directories_created, progress.directories_created
                    );
                }

                if previous_report.total_operations != progress.total_operations {
                    panic!(
                        "invalid progress reported: total_operations must never change \
                        (got change from {} to {})",
                        previous_report.total_operations, progress.total_operations,
                    );
                }

                if previous_report.current_operation_index != progress.current_operation_index {
                    if (previous_report.current_operation_index + 1)
                        != progress.current_operation_index
                    {
                        panic!(
                            "invalid progress reported: current_operation_index must always increase by one \
                            (got change from {} to {})",
                            previous_report.current_operation_index,
                            progress.current_operation_index
                        );
                    }
                } else {
                    match &progress.current_operation {
                        DirectoryMoveOperation::CreatingDirectory { target_path } => {
                            let DirectoryMoveOperation::CreatingDirectory {
                                target_path: previous_target_path,
                            } = &previous_report.current_operation
                            else {
                                panic!(
                                    "invalid progress reported: current_operation changed variant \
                                    without incrementing current_operation_index"
                                );
                            };

                            assert_eq!(target_path, previous_target_path);
                        }
                        DirectoryMoveOperation::CopyingFile { target_path, .. } => {
                            let DirectoryMoveOperation::CopyingFile {
                                target_path: previous_target_path,
                                ..
                            } = &previous_report.current_operation
                            else {
                                panic!(
                                    "invalid progress reported: current_operation changed variant \
                                    without incrementing current_operation_index"
                                );
                            };

                            assert_eq!(target_path, previous_target_path);
                        }
                        DirectoryMoveOperation::RemovingSourceDirectory => {
                            if previous_report.current_operation
                                != DirectoryMoveOperation::RemovingSourceDirectory
                            {
                                panic!(
                                    "invalid progress reported: current_operation changed variant \
                                    without incrementing current_operation_index"
                                );
                            };
                        }
                    }
                }
            }


            last_progress_report = Some(progress.clone());
        },
    ).unwrap();


    let last_progress_report = last_progress_report.unwrap();


    assert_eq!(finished_move.total_bytes_moved, source_scan_bytes);

    assert_eq!(finished_move.total_bytes_moved, last_progress_report.bytes_total);

    assert_eq!(last_progress_report.bytes_total, last_progress_report.bytes_finished);

    assert_eq!(source_scan.files().len(), finished_move.files_moved);

    assert_eq!(source_scan.directories().len(), finished_move.directories_moved);



    deep_harness.assert_not_exists();
    empty_harness.assert_is_directory_and_not_empty();

    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(empty_harness.as_path());


    deep_harness.destroy();
    deep_harness_untouched.destroy();
    empty_harness.destroy();
    Ok(())
}



#[test]
pub fn move_directory_with_progress_errors_when_source_is_symlink_to_destination_directory(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let empty_harness_symlink_path = empty_harness.child_path("some-dir");
    empty_harness_symlink_path.assert_not_exists();
    empty_harness_symlink_path.symlink_to_directory(deep_harness.as_path());

    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());
    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(&empty_harness_symlink_path);


    let move_result: Result<fs_more::directory::DirectoryMoveFinished, MoveDirectoryError> =
        fs_more::directory::move_directory_with_progress(
            &empty_harness_symlink_path,
            deep_harness.as_path(),
            DirectoryMoveWithProgressOptions {
                destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                    existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                    existing_destination_subdirectory_behaviour:
                        ExistingSubDirectoryBehaviour::Abort,
                },
                ..Default::default()
            },
            |_| {},
        );


    assert_matches!(
        move_result.unwrap_err(),
        MoveDirectoryError::PreparationError(
            MoveDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.as_path())
    );


    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());
    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(&empty_harness_symlink_path);


    deep_harness.destroy();
    deep_harness_untouched.destroy();
    empty_harness.destroy();
    Ok(())
}




#[test]
pub fn move_directory_with_progress_does_not_preserve_symlinks_when_destination_directory_already_exists_and_is_not_empty(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let simple_harness = SimpleTree::initialize();

    let deep_harness_non_symlink_copy = DeepTree::initialize();

    let move_destination_harness = SimpleTree::initialize();


    {
        let symlink_point_in_deep_harness = deep_harness.child_path("here-we-go");
        symlink_point_in_deep_harness.assert_not_exists();
        symlink_point_in_deep_harness.symlink_to_directory(simple_harness.as_path());

        fs_more::directory::copy_directory(
            simple_harness.as_path(),
            deep_harness_non_symlink_copy.child_path("here-we-go"),
            DirectoryCopyOptions {
                destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
                copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
            },
        )
        .unwrap();

        symlink_point_in_deep_harness
    };


    deep_harness.assert_is_directory_and_has_contents_of_secondary_directory_with_options(
        deep_harness_non_symlink_copy.as_path(),
        false,
    );



    let finished_move = fs_more::directory::move_directory_with_progress(
        deep_harness.as_path(),
        move_destination_harness.as_path(),
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();

    if let DirectoryMoveStrategy::Rename = finished_move.strategy_used {
        panic!("directory was renamed even though the destination was not empty")
    }


    let remapped_previous_symlink_path = move_destination_harness.child_path("here-we-go");
    remapped_previous_symlink_path.assert_is_directory_and_not_symlink();
    remapped_previous_symlink_path
        .assert_is_directory_and_fully_matches_secondary_directory(simple_harness.as_path());


    move_destination_harness.assert_is_directory_and_has_contents_of_secondary_directory(
        deep_harness_untouched.as_path(),
    );


    move_destination_harness.destroy();
    deep_harness_non_symlink_copy.destroy();
    simple_harness.destroy();
    deep_harness.destroy();
    deep_harness_untouched.destroy();
    Ok(())
}




#[test]
pub fn move_directory_with_progress_may_preserve_symlinks_when_destination_directory_exists_and_is_empty(
) -> TestResult {
    let deep_harness = DeepTree::initialize();
    let simple_harness = SimpleTree::initialize();

    let deep_harness_non_symlink_copy = DeepTree::initialize();

    let copy_destination_harness = EmptyTree::initialize();


    {
        let symlink_point_in_deep_harness = deep_harness.child_path("here-we-go");
        symlink_point_in_deep_harness.assert_not_exists();
        symlink_point_in_deep_harness.symlink_to_directory(simple_harness.as_path());

        fs_more::directory::copy_directory(
            simple_harness.as_path(),
            deep_harness_non_symlink_copy.child_path("here-we-go"),
            DirectoryCopyOptions {
                destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
                copy_depth_limit: CopyDirectoryDepthLimit::Unlimited,
            },
        )
        .unwrap();

        symlink_point_in_deep_harness
    };


    deep_harness.assert_is_directory_and_fully_matches_secondary_directory_with_options(
        deep_harness_non_symlink_copy.as_path(),
        false,
    );



    let finished_move = fs_more::directory::move_directory_with_progress(
        deep_harness.as_path(),
        copy_destination_harness.as_path(),
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    let remapped_here_we_go_dir_path_in_destination =
        copy_destination_harness.child_path("here-we-go");

    match finished_move.strategy_used {
        DirectoryMoveStrategy::Rename => {
            // "here-we-go" inside the destination will be a symlink.

            remapped_here_we_go_dir_path_in_destination.assert_is_symlink_to_directory();
        }
        DirectoryMoveStrategy::CopyAndDelete => {
            // The "here-we-go" directory inside the destination will not have its symlink preserved.
            let remapped_here_we_go_dir_path_in_destination =
                copy_destination_harness.child_path("here-we-go");

            remapped_here_we_go_dir_path_in_destination.assert_is_directory_and_not_symlink();
        }
    }


    remapped_here_we_go_dir_path_in_destination
        .assert_is_directory_and_fully_matches_secondary_directory(simple_harness.as_path());



    copy_destination_harness.destroy();
    deep_harness_non_symlink_copy.destroy();
    simple_harness.destroy();
    deep_harness.destroy();
    Ok(())
}


#[test]
pub fn move_directory_with_progress_performs_merge_without_overwrite_when_copying_to_non_empty_destination_with_correct_options(
) -> TestResult {
    let source_harness = DeepTree::initialize();
    let source_harness_untouched = DeepTree::initialize();

    let destination_harness = SimpleTree::initialize();
    let destination_harness_untouched = SimpleTree::initialize();


    let finished_move = fs_more::directory::move_directory_with_progress(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::CopyAndDelete);

    source_harness.assert_not_exists();

    destination_harness.assert_is_directory_and_has_contents_of_secondary_directory(
        source_harness_untouched.as_path(),
    );
    destination_harness.assert_is_directory_and_has_contents_of_secondary_directory(
        destination_harness_untouched.as_path(),
    );


    source_harness.destroy();
    source_harness_untouched.destroy();
    destination_harness.destroy();
    destination_harness_untouched.destroy();
    Ok(())
}
