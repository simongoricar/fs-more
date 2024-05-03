use assert_matches::assert_matches;
use fs_more::{
    directory::{
        DestinationDirectoryRule,
        DirectoryMoveOptions,
        DirectoryMoveProgress,
        DirectoryMoveWithProgressOptions,
        DirectoryScan,
    },
    error::DirectoryError,
};
use fs_more_test_harness::{
    assertable::AssertableDirectoryPath,
    error::TestResult,
    trees::{DeepTreeHarness, EmptyTreeHarness},
};


#[test]
pub fn move_directory() -> TestResult<()> {
    let harness_for_comparison = DeepTreeHarness::new()?;
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), None, false).unwrap();
    let source_size_bytes = source_scan.total_size_in_bytes().unwrap();

    empty_harness.root.assert_is_empty();

    let move_result = fs_more::directory::move_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryMoveOptions {
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
        .assert_directory_contents_match_directory(empty_harness.root.path());

    empty_harness.destroy()?;
    // No need to destroy `harness` as the directory no longer exists due to being moved.
    Ok(())
}

#[test]
pub fn error_on_move_directory_with_source_symlink_to_same_directory() -> TestResult<()> {
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
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                create_missing_subdirectories: true,
                overwrite_existing_files: true,
            },
        },
    );

    assert_matches!(
        move_result,
        Err(DirectoryError::InvalidDestinationDirectoryPath)
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
pub fn error_on_move_directory_with_progress_with_source_symlink_to_same_directory(
) -> TestResult<()> {
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
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                create_missing_subdirectories: true,
                overwrite_existing_files: true,
            },
            ..Default::default()
        },
        |_| {},
    );

    assert_matches!(
        move_result,
        Err(DirectoryError::InvalidDestinationDirectoryPath)
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
pub fn move_directory_with_source_symlink_to_different_directory() -> TestResult<()> {
    let symlink_target_harness = DeepTreeHarness::new()?;
    let untouched_harness_copy = DeepTreeHarness::new()?;
    let source_symlink_harness = EmptyTreeHarness::new()?;
    let copy_target_harness = EmptyTreeHarness::new()?;

    source_symlink_harness.root.assert_is_empty();
    copy_target_harness.root.assert_is_empty();


    let source_scan =
        DirectoryScan::scan_with_options(symlink_target_harness.root.path(), None, false).unwrap();
    let source_size_bytes = source_scan.total_size_in_bytes().unwrap();


    let symlink_path =
        AssertableDirectoryPath::from_path(source_symlink_harness.root.child_path("test-symlink"));
    symlink_path.symlink_to_directory(symlink_target_harness.root.path())?;

    symlink_path.assert_is_symlink_to_directory();
    let symlink_target = symlink_path.resolve_target_symlink_path();

    assert_eq!(
        symlink_target.as_path(),
        symlink_target_harness.root.path()
    );

    // Now attempt to perform a move - it should be fine.
    let move_result = fs_more::directory::move_directory(
        symlink_path.path(),
        copy_target_harness.root.path(),
        DirectoryMoveOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                create_missing_subdirectories: true,
                overwrite_existing_files: true,
            },
        },
    );

    let directory_move = move_result.unwrap();


    assert_eq!(
        source_size_bytes, directory_move.total_bytes_moved,
        "move_directory reported incorrect amount of bytes moved"
    );

    symlink_target_harness.root.assert_not_exists();
    untouched_harness_copy
        .root
        .assert_directory_contents_match_directory(copy_target_harness.root.path());


    source_symlink_harness.destroy()?;
    untouched_harness_copy.destroy()?;

    Ok(())
}

#[test]
pub fn move_directory_with_progress_with_source_symlink_to_different_directory() -> TestResult<()> {
    let symlink_target_harness = DeepTreeHarness::new()?;
    let untouched_harness_copy = DeepTreeHarness::new()?;
    let source_symlink_harness = EmptyTreeHarness::new()?;
    let copy_target_harness = EmptyTreeHarness::new()?;

    source_symlink_harness.root.assert_is_empty();
    copy_target_harness.root.assert_is_empty();


    let source_scan =
        DirectoryScan::scan_with_options(symlink_target_harness.root.path(), None, false).unwrap();
    let source_size_bytes = source_scan.total_size_in_bytes().unwrap();


    let symlink_path =
        AssertableDirectoryPath::from_path(source_symlink_harness.root.child_path("test-symlink"));
    symlink_path.symlink_to_directory(symlink_target_harness.root.path())?;

    symlink_path.assert_is_symlink_to_directory();
    let symlink_target = symlink_path.resolve_target_symlink_path();

    assert_eq!(
        symlink_target.as_path(),
        symlink_target_harness.root.path()
    );

    // Now attempt to perform a move - it should be fine.
    let move_result = fs_more::directory::move_directory_with_progress(
        symlink_path.path(),
        copy_target_harness.root.path(),
        DirectoryMoveWithProgressOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                create_missing_subdirectories: true,
                overwrite_existing_files: true,
            },
            ..Default::default()
        },
        |_| {},
    );

    let directory_move = move_result.unwrap();


    assert_eq!(
        source_size_bytes, directory_move.total_bytes_moved,
        "move_directory reported incorrect amount of bytes moved"
    );

    symlink_target_harness.root.assert_not_exists();
    untouched_harness_copy
        .root
        .assert_directory_contents_match_directory(copy_target_harness.root.path());


    source_symlink_harness.destroy()?;
    untouched_harness_copy.destroy()?;

    Ok(())
}

#[test]
pub fn move_directory_with_progress() -> TestResult<()> {
    let harness_for_comparison = DeepTreeHarness::new()?;
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan = DirectoryScan::scan_with_options(harness.root.path(), None, false).unwrap();
    let source_size_bytes = source_scan.total_size_in_bytes().unwrap();

    empty_harness.root.assert_is_empty();


    let mut last_report: Option<DirectoryMoveProgress> = None;

    let move_result = fs_more::directory::move_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryMoveWithProgressOptions {
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
