use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use fs_more::directory::DirectoryScanDepthLimit;
use fs_more_test_harness::{
    assertable::{r#trait::ManageablePath, AsPath},
    error::TestResult,
    tree_framework::FileSystemHarness,
    trees::simple::SimpleTree,
};



/// Asserts that all paths in the `scanned_paths` iterator
/// appear in the `expected_set_of_paths` iterator (order is ignored).
///
/// If a path is missing, this function panics with the details.
fn assert_path_list_fully_matches_set<S, SP, D, DP>(scanned_paths: S, expected_set_of_paths: D)
where
    S: IntoIterator<Item = SP>,
    SP: AsRef<Path>,
    D: IntoIterator<Item = DP>,
    DP: AsRef<Path>,
{
    let mut scanned_path_set: HashSet<PathBuf> = HashSet::from_iter(
        scanned_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );

    let expected_path_set: HashSet<PathBuf> = HashSet::from_iter(
        expected_set_of_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );


    let _ = scanned_path_set.drain().map(|scanned_path| {
        if !expected_path_set.contains(&scanned_path) {
            panic!(
                "path \"{}\" was not present in scanned paths",
                scanned_path.display()
            );
        }
    });
}



#[test]
pub fn directory_scan_produces_correct_information() -> TestResult {
    let harness = SimpleTree::initialize();


    let scan = fs_more::directory::DirectoryScan::scan_with_options(
        harness.as_path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .unwrap();


    assert_eq!(scan.directories().len(), 1);
    assert_eq!(scan.files().len(), 3);

    assert!(scan.covers_entire_directory_tree());


    assert_path_list_fully_matches_set(scan.directories(), [harness.foo.as_path()]);
    assert_path_list_fully_matches_set(
        scan.files(),
        [
            harness.empty_txt.as_path(),
            harness.foo.hello_world_txt.as_path(),
            harness.foo.bar_bin.as_path(),
        ],
    );


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_respects_limited_depth_option() -> TestResult {
    let harness = SimpleTree::initialize();


    let scan = fs_more::directory::DirectoryScan::scan_with_options(
        harness.as_path(),
        DirectoryScanDepthLimit::Limited { maximum_depth: 0 },
        false,
    )
    .unwrap();


    assert_eq!(scan.directories().len(), 1);
    assert_eq!(scan.files().len(), 1);

    assert!(!scan.covers_entire_directory_tree());


    assert_path_list_fully_matches_set(scan.directories(), [harness.foo.as_path()]);
    assert_path_list_fully_matches_set(scan.files(), [harness.empty_txt.as_path()]);


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_calculates_correct_size() -> TestResult {
    let harness = SimpleTree::initialize();


    let actual_size_of_harness_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.foo.size_in_bytes();
        let hello_world_size_bytes = harness.foo.hello_world_txt.size_in_bytes();
        let bar_bin_size_bytes = harness.foo.bar_bin.size_in_bytes();

        empty_txt_size_bytes + foo_dir_size_bytes + hello_world_size_bytes + bar_bin_size_bytes
    };


    let scan = fs_more::directory::DirectoryScan::scan_with_options(
        harness.as_path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .unwrap();


    assert!(scan.covers_entire_directory_tree());


    let scanned_size_in_bytes = scan.total_size_in_bytes().unwrap();
    assert_eq!(
        scanned_size_in_bytes,
        actual_size_of_harness_in_bytes
    );


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_calculates_correct_size_with_depth_limit() -> TestResult<()> {
    let harness = SimpleTree::initialize();


    let actual_size_of_scan_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.foo.size_in_bytes();

        empty_txt_size_bytes + foo_dir_size_bytes
    };


    let scan = fs_more::directory::DirectoryScan::scan_with_options(
        harness.as_path(),
        DirectoryScanDepthLimit::Limited { maximum_depth: 0 },
        false,
    )
    .unwrap();


    assert!(!scan.covers_entire_directory_tree());

    let scanned_size_in_bytes = scan.total_size_in_bytes().unwrap();
    assert_eq!(
        scanned_size_in_bytes,
        actual_size_of_scan_in_bytes
    );


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_size_in_bytes_produces_correct_information() -> TestResult {
    let harness = SimpleTree::initialize();


    let actual_size_of_harness_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.foo.size_in_bytes();
        let hello_world_size_bytes = harness.foo.hello_world_txt.size_in_bytes();
        let bar_bin_size_bytes = harness.foo.bar_bin.size_in_bytes();

        empty_txt_size_bytes + foo_dir_size_bytes + hello_world_size_bytes + bar_bin_size_bytes
    };


    let directory_size_bytes =
        fs_more::directory::directory_size_in_bytes(harness.as_path(), false).unwrap();

    assert_eq!(
        directory_size_bytes,
        actual_size_of_harness_in_bytes
    );


    harness.destroy();
    Ok(())
}
