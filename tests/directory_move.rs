use fs_more::directory::{
    DirectoryMoveOptions,
    DirectoryMoveProgress,
    DirectoryMoveWithProgressOptions,
    DirectoryScan,
    TargetDirectoryRule,
};
use fs_more_test_harness::{
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
            target_directory_rule: TargetDirectoryRule::AllowEmpty,
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

// TODO Add a test for behaviour when moving "symlink to directory A" to "A".

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
            target_directory_rule: TargetDirectoryRule::AllowEmpty,
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
