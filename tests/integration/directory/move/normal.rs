use assert_matches::assert_matches;
use fs_more::{
    directory::{
        directory_size_in_bytes,
        DestinationDirectoryRule,
        DirectoryMoveStrategy,
        DirectoryScan,
        DirectoryScanDepthLimit,
        ExistingSubDirectoryBehaviour,
        MoveDirectoryOptions,
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
    trees::{DeepTreeHarness, EmptyTreeHarness, SimpleTreeHarness},
};



#[test]
pub fn move_directory_moves_all_files_and_subdirectories() -> TestResult {
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

    let move_result = fs_more::directory::move_directory(
        harness.root.path(),
        empty_harness.root.path(),
        MoveDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
        },
    );

    let move_details = move_result.unwrap();

    assert_eq!(
        source_size_bytes, move_details.total_bytes_moved,
        "move_directory reported incorrect amount of bytes moved"
    );


    harness.root.assert_not_exists();
    empty_harness.root.assert_is_not_empty();

    harness_for_comparison
        .root
        .assert_directory_contents_fully_match_directory(empty_harness.root.path());

    empty_harness.destroy()?;
    // No need to destroy `harness` as the directory no longer exists due to being moved.
    Ok(())
}



#[test]
pub fn move_directory_errors_when_source_is_symlink_to_destination_directory() -> TestResult {
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
    let move_result = fs_more::directory::move_directory(
        symlink_path.path(),
        target_harness.root.path(),
        MoveDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
            },
        },
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
        .assert_directory_contents_fully_match_directory(target_harness.root.path());

    source_symlink_harness.destroy()?;
    target_harness.destroy()?;
    target_harness_untouched.destroy()?;

    Ok(())
}



#[test]
pub fn move_directory_does_not_preserve_symlinks_when_destination_directory_already_exists_and_is_not_empty(
) -> TestResult {
    let symlink_source_harness = EmptyTreeHarness::new()?;
    let symlink_source_harness_untouched_copy = EmptyTreeHarness::new()?;
    symlink_source_harness.root.assert_is_empty();

    let symlink_target_harness = DeepTreeHarness::new()?;
    symlink_target_harness.root.assert_is_directory();
    symlink_target_harness.root.assert_is_not_empty();

    let untouched_copy_of_symlink_target_harness = DeepTreeHarness::new()?;
    untouched_copy_of_symlink_target_harness
        .root
        .assert_is_not_empty();

    let directory_copy_destination_harness = SimpleTreeHarness::new()?;


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


    let move_destination =
        AssertableDirectoryPath::from_path(directory_copy_destination_harness.root.path());

    symlink_target.assert_is_directory();

    let directory_move_result = fs_more::directory::move_directory(
        symlink.path(),
        move_destination.path(),
        MoveDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
        },
    )
    .unwrap();


    println!("{:?}", directory_move_result);

    symlink_target.assert_is_directory();

    move_destination.assert_directory_has_contents_of_other_directory(
        untouched_copy_of_symlink_target_harness.root.path(),
    );
    move_destination.assert_directory_has_contents_of_other_directory(
        symlink_source_harness_untouched_copy.root.path(),
    );

    assert_eq!(
        directory_move_result.total_bytes_moved,
        symlink_target_harness_total_size_bytes
    );


    match directory_move_result.strategy_used {
        DirectoryMoveStrategy::Rename => {
            panic!("Directory was renamed, even though the destination contained some contents.");
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
pub fn move_directory_may_preserve_symlinks_when_destination_directory_exists_and_is_empty(
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


    let directory_move_result = fs_more::directory::move_directory(
        symlink.path(),
        move_destination.path(),
        MoveDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
        },
    )
    .unwrap();


    symlink.assert_not_exists();
    untouched_copy_of_target_harness
        .root
        .assert_directory_contents_fully_match_directory(symlink_target.path());

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



#[test]
pub fn move_directory_performs_merge_without_overwrite_when_copying_to_non_empty_destination_with_correct_options(
) -> TestResult {
    let move_source_harness = SimpleTreeHarness::new()?;
    let source_harness_for_comparison = SimpleTreeHarness::new()?;

    let move_destination_harness = DeepTreeHarness::new()?;
    let destination_harness_for_comparison = DeepTreeHarness::new()?;


    move_source_harness.root.assert_is_directory();


    let directory_move_result = fs_more::directory::move_directory(
        move_source_harness.root.path(),
        move_destination_harness.root.path(),
        MoveDirectoryOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
                existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Abort,
            },
        },
    )
    .unwrap();


    move_source_harness.root.assert_not_exists();

    move_destination_harness
        .root
        .assert_directory_has_contents_of_other_directory(
            source_harness_for_comparison.root.path(),
        );

    move_destination_harness
        .root
        .assert_directory_has_contents_of_other_directory(
            destination_harness_for_comparison.root.path(),
        );

    assert!(directory_move_result.strategy_used == DirectoryMoveStrategy::CopyAndDelete);


    move_destination_harness.destroy()?;
    source_harness_for_comparison.destroy()?;
    destination_harness_for_comparison.destroy()?;
    move_source_harness.destroy()?;
    Ok(())
}
