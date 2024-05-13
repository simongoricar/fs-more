use assert_matches::assert_matches;
use fs_more::{
    directory::{
        directory_size_in_bytes,
        DestinationDirectoryRule,
        DirectoryMoveProgress,
        DirectoryMoveStrategy,
        DirectoryScan,
        DirectoryScanDepthLimit,
        ExistingSubDirectoryBehaviour,
        MoveDirectoryWithProgressOptions,
    },
    error::{
        DestinationDirectoryPathValidationError,
        MoveDirectoryError,
        MoveDirectoryPreparationError,
    },
    file::ExistingFileBehaviour,
};
use fs_more_test_harness::{
    assertable::AssertableDirectoryPath,
    error::TestResult,
    trees::{DeepTreeHarness, EmptyTreeHarness},
};



#[test]
pub fn error_on_move_directory_with_progress_using_source_symlink_to_destination_directory(
) -> TestResult {
    let target_harness = DeepTreeHarness::new()?;
    let target_harness_untouched = DeepTreeHarness::new()?;
    let source_symlink_harness = EmptyTreeHarness::new()?;

    source_symlink_harness.root.assert_is_empty();

    let symlink_path =
        AssertableDirectoryPath::from_path(source_symlink_harness.root.child_path("test-symlink"));
    symlink_path.symlink_to_directory(target_harness.root.path())?;

    symlink_path.assert_is_symlink_to_directory();
    let symlink_target = symlink_path.resolve_target_symlink_path();

    assert_eq!(
        symlink_target.as_path(),
        target_harness.root.path()
    );


    // Now attempt to perform a move - it should error.
    let move_result = fs_more::directory::move_directory_with_progress(
        symlink_path.path(),
        target_harness.root.path(),
        MoveDirectoryWithProgressOptions {
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
        move_result.unwrap_err(),
        MoveDirectoryError::PreparationError(
            MoveDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        ) if source_directory_path == target_harness.root.path() && destination_directory_path == target_harness.root.path()
    );

    target_harness_untouched
        .root
        .assert_directory_contents_match_directory(target_harness.root.path());

    source_symlink_harness.destroy()?;
    target_harness.destroy()?;
    target_harness_untouched.destroy()?;

    Ok(())
}



#[test]
pub fn move_directory_with_progress() -> TestResult {
    let harness_for_comparison = DeepTreeHarness::new()?;
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(
        harness.root.path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .unwrap();
    let source_size_bytes = source_scan.total_size_in_bytes().unwrap();

    empty_harness.root.assert_is_empty();


    let mut last_report: Option<DirectoryMoveProgress> = None;

    let move_result = fs_more::directory::move_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        MoveDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |progress| {
            let Some(previous_progress) = &last_report else {
                last_report = Some(progress.clone());
                return;
            };

            // Ensure `bytes_total` and `total_operations` don't change.

            if progress.bytes_total != previous_progress.bytes_total {
                panic!(
                    "move_directory_with_progress made two consecutive progress reports \
                    where bytes_total changed"
                );
            }

            if progress.total_operations != previous_progress.total_operations {
                panic!(
                    "move_directory_with_progress made two consecutive progress reports \
                    where total_operations changed"
                );
            }

            // Ensure `bytes_finished`, `files_moved` and `directories_created`  are monotonically increasing.

            if progress.bytes_finished < previous_progress.bytes_finished {
                panic!(
                    "move_directory_with_progress made two consecutive progress reports \
                    where the second had a smaller bytes_finished"
                );
            }

            if progress.files_moved < previous_progress.files_moved {
                panic!(
                    "move_directory_with_progress made two consecutive progress reports \
                    where the second had a smaller files_moved"
                );
            }

            if progress.directories_created < previous_progress.directories_created {
                panic!(
                    "move_directory_with_progress made two consecutive progress reports \
                    where the second had a smaller directories_created"
                );
            }


            last_report = Some(progress.clone());
        },
    );

    let move_details = move_result.unwrap();

    assert_eq!(
        source_size_bytes, move_details.total_bytes_moved,
        "move_directory_with_progress reported incorrect amount of bytes moved"
    );


    harness.root.assert_not_exists();
    empty_harness.root.assert_is_not_empty();

    harness_for_comparison
        .root
        .assert_directory_contents_match_directory(empty_harness.root.path());

    empty_harness.destroy()?;
    // No need to destroy `harness` as the directory no longer exists due to being moved.
    Ok(())
}



#[test]
pub fn move_directory_with_progress_source_directory_symlink_behaviour_with_existing_empty_destination_directory(
) -> TestResult {
    let symlink_source_harness = EmptyTreeHarness::new()?;
    symlink_source_harness.root.assert_is_empty();

    let symlink_target_harness = DeepTreeHarness::new()?;
    symlink_target_harness.root.assert_is_directory();
    symlink_target_harness.root.assert_is_not_empty();

    let untouched_copy_of_symlink_target_harness = DeepTreeHarness::new()?;
    untouched_copy_of_symlink_target_harness
        .root
        .assert_is_not_empty();

    let directory_copy_destination_harness = EmptyTreeHarness::new()?;
    directory_copy_destination_harness.root.assert_is_empty();


    let symlink_target_harness_total_size_bytes =
        directory_size_in_bytes(symlink_target_harness.root.path(), true).unwrap();


    let symlink =
        AssertableDirectoryPath::from_path(symlink_source_harness.root.child_path("symlink"));
    let symlink_target = AssertableDirectoryPath::from_path(symlink_target_harness.root.path());

    symlink.symlink_to_directory(symlink_target.path())?;

    {
        symlink.assert_is_symlink_to_directory();
        let symlink_resolved_target = symlink.resolve_target_symlink_path();
        assert_eq!(
            symlink_resolved_target.as_path(),
            symlink_target.path()
        );
    }

    untouched_copy_of_symlink_target_harness
        .root
        .assert_directory_contents_match_directory(symlink_target.path());


    let move_destination =
        AssertableDirectoryPath::from_path(directory_copy_destination_harness.root.path());

    symlink_target.assert_is_directory();

    let directory_move_result = fs_more::directory::move_directory_with_progress(
        symlink.path(),
        move_destination.path(),
        MoveDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    println!("{:?}", directory_move_result);

    symlink_target.assert_is_directory();
    untouched_copy_of_symlink_target_harness
        .root
        .assert_directory_contents_match_directory(symlink_target.path());

    assert_eq!(
        directory_move_result.total_bytes_moved,
        symlink_target_harness_total_size_bytes
    );


    match directory_move_result.strategy_used {
        DirectoryMoveStrategy::Rename => {
            // Destination will be a symlink.
            move_destination.assert_is_symlink_to_directory();

            let resolved_path = move_destination.resolve_target_symlink_path();

            assert_eq!(resolved_path.as_path(), symlink_target.path());
        }
        DirectoryMoveStrategy::CopyAndDelete => {
            // Copy and delete will not preserve the symlink.
            move_destination.assert_is_directory();
        }
    };


    symlink_source_harness.destroy()?;
    symlink_target_harness.destroy()?;
    untouched_copy_of_symlink_target_harness.destroy()?;
    directory_copy_destination_harness.destroy()?;
    Ok(())
}



#[test]
pub fn move_directory_with_progress_source_directory_symlink_behaviour_without_existing_destination_directory(
) -> TestResult {
    let symlink_source_harness = EmptyTreeHarness::new()?;
    symlink_source_harness.root.assert_is_empty();

    let symlink_target_harness = DeepTreeHarness::new()?;
    symlink_target_harness.root.assert_is_not_empty();

    let untouched_copy_of_target_harness = DeepTreeHarness::new()?;
    untouched_copy_of_target_harness.root.assert_is_not_empty();

    let directory_copy_destination_harness = EmptyTreeHarness::new()?;
    directory_copy_destination_harness.root.assert_is_empty();


    let symlink_target_harness_total_size_bytes =
        directory_size_in_bytes(symlink_target_harness.root.path(), true).unwrap();


    let symlink =
        AssertableDirectoryPath::from_path(symlink_source_harness.root.child_path("symlink"));
    let symlink_target = AssertableDirectoryPath::from_path(symlink_target_harness.root.path());

    symlink.symlink_to_directory(symlink_target.path())?;

    {
        symlink.assert_is_symlink_to_directory();
        let symlink_target = symlink.resolve_target_symlink_path();
        assert_eq!(
            symlink_target.as_path(),
            symlink_target_harness.root.path()
        );
    }


    let move_destination =
        AssertableDirectoryPath::from_path(directory_copy_destination_harness.root.path());


    let directory_move_result = fs_more::directory::move_directory_with_progress(
        symlink.path(),
        move_destination.path(),
        MoveDirectoryWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();


    symlink.assert_not_exists();
    untouched_copy_of_target_harness
        .root
        .assert_directory_contents_match_directory(symlink_target.path());

    assert_eq!(
        directory_move_result.total_bytes_moved,
        symlink_target_harness_total_size_bytes
    );


    match directory_move_result.strategy_used {
        DirectoryMoveStrategy::Rename => {
            // Destination will be a symlink.
            move_destination.assert_is_symlink_to_directory();

            let resolved_path = move_destination.resolve_target_symlink_path();

            assert_eq!(resolved_path.as_path(), move_destination.path(),);
        }
        DirectoryMoveStrategy::CopyAndDelete => {
            // Copy and delete will not preserve the symlink.
            move_destination.assert_is_directory();
        }
    };


    symlink_source_harness.destroy()?;
    symlink_target_harness.destroy()?;
    untouched_copy_of_target_harness.destroy()?;
    directory_copy_destination_harness.destroy()?;
    Ok(())
}