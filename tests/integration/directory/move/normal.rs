use fs_more::{
    directory::{
        CopyDirectoryDepthLimit,
        DestinationDirectoryRule,
        DirectoryCopyOptions,
        DirectoryMoveOptions,
        DirectoryMoveStrategy,
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
    collect_directory_statistics_via_scan,
    prelude::*,
    trees::structures::{deep::DeepTree, empty::EmptyTree, simple::SimpleTree},
};



#[test]
pub fn move_directory_moves_all_files_and_subdirectories() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let source_harness_stats =
        collect_directory_statistics_via_scan(deep_harness.as_path()).unwrap();



    let finished_move = fs_more::directory::move_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
    )
    .unwrap();


    assert_eq!(finished_move.total_bytes_moved, source_harness_stats.total_bytes);


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
pub fn move_directory_errors_when_source_is_symlink_to_destination_directory() -> TestResult {
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


    let move_result = fs_more::directory::move_directory(
        &empty_harness_symlink_path,
        deep_harness.as_path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
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
pub fn move_directory_does_not_preserve_symlinks_when_destination_directory_already_exists_and_is_not_empty(
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



    let finished_move = fs_more::directory::move_directory(
        deep_harness.as_path(),
        move_destination_harness.as_path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
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
pub fn move_directory_may_preserve_symlinks_when_destination_directory_exists_and_is_empty(
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



    let finished_move = fs_more::directory::move_directory(
        deep_harness.as_path(),
        copy_destination_harness.as_path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    )
    .unwrap();


    let remapped_here_we_go_dir_path_in_destination =
        copy_destination_harness.child_path("here-we-go");

    match finished_move.strategy_used {
        DirectoryMoveStrategy::Rename => {
            // "here-we-go" inside the destination will be a symlink.

            remapped_here_we_go_dir_path_in_destination.assert_is_valid_symlink_to_directory();
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
pub fn move_directory_performs_merge_without_overwrite_when_copying_to_non_empty_destination_with_correct_options(
) -> TestResult {
    let source_harness = DeepTree::initialize();
    let source_harness_untouched = DeepTree::initialize();

    let destination_harness = SimpleTree::initialize();
    let destination_harness_untouched = SimpleTree::initialize();


    let finished_move = fs_more::directory::move_directory(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
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
fn move_directory_renames_source_to_destination_when_destination_does_not_exist() {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();

    let destination_directory_path = destination_harness.child_path("inner-destination");
    destination_directory_path.assert_not_exists();


    let finished_move = fs_more::directory::move_directory(
        source_harness.as_path(),
        &destination_directory_path,
        DirectoryMoveOptions::default(),
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::Rename);


    destination_harness.destroy();
    source_harness.destroy();
}



#[test]
#[cfg(unix)]
fn move_directory_renames_source_to_destination_when_destination_is_empty_on_unix() {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();


    let finished_move = fs_more::directory::move_directory(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveOptions::default(),
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::Rename);


    destination_harness.destroy();
    source_harness.destroy();
}



#[test]
#[cfg(windows)]
fn move_directory_does_not_rename_source_to_destination_when_destination_is_empty_on_windows() {
    let source_harness = SimpleTree::initialize();
    let destination_harness = EmptyTree::initialize();


    let finished_move = fs_more::directory::move_directory(
        source_harness.as_path(),
        destination_harness.as_path(),
        DirectoryMoveOptions::default(),
    )
    .unwrap();


    assert_eq!(finished_move.strategy_used, DirectoryMoveStrategy::CopyAndDelete);


    destination_harness.destroy();
    source_harness.destroy();
}


// TODO Revisit tests that handle symlinks: remove obsolete tests and add new ones that test the symlink options.
