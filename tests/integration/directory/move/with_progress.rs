use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        DestinationDirectoryRule,
        DirectoryCopyOptions,
        DirectoryMoveOperation,
        DirectoryMoveProgress,
        DirectoryMoveStrategy,
        DirectoryMoveWithProgressAllowedStrategies,
        DirectoryMoveWithProgressByCopyOptions,
        DirectoryMoveWithProgressOptions,
        ExistingSubDirectoryBehaviour,
        SymlinkBehaviour,
    },
    error::{
        DestinationDirectoryPathValidationError,
        MoveDirectoryError,
        MoveDirectoryPreparationError,
    },
    file::ExistingFileBehaviour,
};
use fs_more_test_harness::{
    collect_directory_statistics_via_scan,
    prelude::*,
    trees::structures::{deep::DeepTree, empty::EmptyTree, simple::SimpleTree},
};



#[test]
pub fn move_directory_with_progress_moves_all_files_and_subdirectories() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let source_harness_stats =
        collect_directory_statistics_via_scan(deep_harness.as_path()).unwrap();



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

                        DirectoryMoveOperation::CreatingSymbolicLink { destination_symbolic_link_file_path } => {
                            let DirectoryMoveOperation::CreatingSymbolicLink { destination_symbolic_link_file_path: previous_destination_symbolic_link_file_path } = &previous_report.current_operation else {
                                panic!(
                                    "invalid progress reported: current_operation changed variant \
                                    without incrementing current_operation_index"
                                );
                            };

                            assert_eq!(destination_symbolic_link_file_path, previous_destination_symbolic_link_file_path);
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


    assert_eq!(finished_move.total_bytes_moved, source_harness_stats.total_bytes);

    assert_eq!(finished_move.total_bytes_moved, last_progress_report.bytes_total);

    assert_eq!(last_progress_report.bytes_total, last_progress_report.bytes_finished);

    assert_eq!(source_harness_stats.total_files, finished_move.files_moved);

    assert_eq!(
        source_harness_stats.total_directories,
        finished_move.directories_moved
    );



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
pub fn move_directory_with_progress_preserves_symlinks_on_non_empty_destination_directory_with_only_rename_strategy_enabled(
) {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let simple_harness = SimpleTree::initialize();

    let deep_harness_non_symlink_copy = DeepTree::initialize();

    let move_destination_harness = EmptyTree::initialize();

    // We need this on Windows, because for a rename to work, the destination directory must not exist.
    move_destination_harness.assert_is_empty_directory_and_remove();


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
                ..Default::default()
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
            allowed_strategies: DirectoryMoveWithProgressAllowedStrategies::OnlyRename,
        },
        |_| {},
    )
    .unwrap();

    if finished_move.strategy_used != DirectoryMoveStrategy::Rename {
        panic!("directory was copy-and-deleted even though that strategy was disabled");
    }


    let remapped_previous_symlink_path = move_destination_harness.child_path("here-we-go");
    let resolved_remapped_previous_symlink_path = remapped_previous_symlink_path
        .assert_is_valid_symlink_to_directory_and_resolve_destination();

    resolved_remapped_previous_symlink_path
        .assert_is_directory_and_fully_matches_secondary_directory(simple_harness.as_path());


    move_destination_harness.assert_is_directory_and_has_contents_of_secondary_directory(
        deep_harness_untouched.as_path(),
    );


    move_destination_harness.destroy();
    deep_harness_non_symlink_copy.destroy();
    simple_harness.destroy();
    deep_harness.destroy();
    deep_harness_untouched.destroy();
}




#[test]
pub fn move_directory_does_not_preserve_symlinks_on_empty_destination_directory_with_only_copy_and_delete_strategy_and_symlink_following(
) {
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
                ..Default::default()
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
            allowed_strategies: DirectoryMoveWithProgressAllowedStrategies::OnlyCopyAndDelete {
                options: DirectoryMoveWithProgressByCopyOptions {
                    symlink_behaviour: SymlinkBehaviour::Follow,
                    ..Default::default()
                },
            },
        },
        |_| {},
    )
    .unwrap();


    if finished_move.strategy_used != DirectoryMoveStrategy::CopyAndDelete {
        panic!("directory was renamed even though that strategy was disabled");
    }


    let remapped_here_we_go_dir_path_in_destination =
        copy_destination_harness.child_path("here-we-go");

    // The "here-we-go" directory inside the destination will not have its symlink preserved.
    remapped_here_we_go_dir_path_in_destination.assert_is_directory_and_not_symlink();
    remapped_here_we_go_dir_path_in_destination
        .assert_is_directory_and_fully_matches_secondary_directory(simple_harness.as_path());



    copy_destination_harness.destroy();
    deep_harness_non_symlink_copy.destroy();
    simple_harness.destroy();
    deep_harness.destroy();
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



#[test]
fn move_directory_with_progress_renames_source_to_destination_when_destination_does_not_exist() {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();

    let destination_directory_path = destination_harness.child_path("inner-destination");
    destination_directory_path.assert_not_exists();


    let finished_move = fs_more::directory::move_directory_with_progress(
        source_harness.as_path(),
        &destination_directory_path,
        DirectoryMoveWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::Rename);


    destination_harness.destroy();
    source_harness.destroy();
}



#[test]
#[cfg(unix)]
fn move_directory_with_progress_renames_source_to_destination_when_destination_is_empty_on_unix() {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();


    let finished_move = fs_more::directory::move_directory_with_progress(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::Rename);


    destination_harness.destroy();
    source_harness.destroy();
}



#[test]
#[cfg(windows)]
fn move_directory_with_progress_does_not_rename_source_to_destination_when_destination_is_empty_on_windows(
) {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();


    let finished_move = fs_more::directory::move_directory_with_progress(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveWithProgressOptions::default(),
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::CopyAndDelete);


    destination_harness.destroy();
    source_harness.destroy();
}



#[test]
fn move_directory_with_progress_preserves_source_directory_symlink_when_using_rename_strategy() {
    let harness_with_only_symlink = EmptyTree::initialize();
    let simple_tree = SimpleTree::initialize();
    let destination_tree = EmptyTree::initialize();


    let (symlink_to_simple_tree_under_source, symlink_to_simple_tree_under_destination) = {
        let symlink_to_simple_tree_under_source =
            harness_with_only_symlink.child_path("symlink-to-simple");
        symlink_to_simple_tree_under_source.assert_not_exists();
        symlink_to_simple_tree_under_source.symlink_to_directory(simple_tree.as_path());

        let symlink_to_simple_tree_under_destination =
            destination_tree.child_path("symlink-to-simple");
        symlink_to_simple_tree_under_destination.assert_not_exists();


        (
            symlink_to_simple_tree_under_source,
            symlink_to_simple_tree_under_destination,
        )
    };


    let finished_move = fs_more::directory::move_directory_with_progress(
        &symlink_to_simple_tree_under_source,
        &symlink_to_simple_tree_under_destination,
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
            allowed_strategies: DirectoryMoveWithProgressAllowedStrategies::OnlyRename,
        },
        |_| {},
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::Rename);


    symlink_to_simple_tree_under_source.assert_not_exists();
    simple_tree.assert_is_directory_and_not_empty();
    symlink_to_simple_tree_under_destination
        .assert_is_valid_symlink_to_directory_and_destination_matches(simple_tree.as_path());


    harness_with_only_symlink.destroy();
    simple_tree.destroy();
}
