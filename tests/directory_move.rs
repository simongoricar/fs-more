use fs_more::directory::{DirectoryMoveOptions, DirectoryScan, TargetDirectoryRule};
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
