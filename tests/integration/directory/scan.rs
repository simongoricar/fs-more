use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use fs_more::directory::DirectoryScanDepthLimit;
use fs_more_test_harness::{
    assertable::{
        r#trait::{AssertablePath, ManageablePath},
        AsPath,
    },
    error::TestResult,
    tree_framework::{FileSystemHarness, FileSystemHarnessDirectory},
    trees::{deep::DeepTree, simple::SimpleTree},
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
    let scanned_path_set: HashSet<PathBuf> = HashSet::from_iter(
        scanned_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );

    let expected_path_set: HashSet<PathBuf> = HashSet::from_iter(
        expected_set_of_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );


    for scanned_path in scanned_path_set.iter() {
        if !expected_path_set.contains(scanned_path.as_path()) {
            panic!(
                "path \"{}\" was scanned, but not present in expected paths",
                scanned_path.display()
            );
        }
    }

    for expected_path in expected_path_set.iter() {
        if !scanned_path_set.contains(expected_path.as_path()) {
            panic!(
                "path \"{}\" was expected, but not present in scanned paths",
                expected_path.display()
            );
        }
    }
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


    assert_path_list_fully_matches_set(scan.directories(), [harness.yes.as_path()]);
    assert_path_list_fully_matches_set(
        scan.files(),
        [
            harness.empty_txt.as_path(),
            harness.yes.hello_world_txt.as_path(),
            harness.yes.no_bin.as_path(),
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


    assert_path_list_fully_matches_set(scan.directories(), [harness.yes.as_path()]);
    assert_path_list_fully_matches_set(scan.files(), [harness.empty_txt.as_path()]);


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_calculates_correct_size() -> TestResult {
    let harness = SimpleTree::initialize();


    let actual_size_of_harness_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.yes.size_in_bytes();
        let hello_world_size_bytes = harness.yes.hello_world_txt.size_in_bytes();
        let bar_bin_size_bytes = harness.yes.no_bin.size_in_bytes();

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
    assert_eq!(scanned_size_in_bytes, actual_size_of_harness_in_bytes);


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_calculates_correct_size_with_depth_limit() -> TestResult<()> {
    let harness = SimpleTree::initialize();


    let actual_size_of_scan_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.yes.size_in_bytes();

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
    assert_eq!(scanned_size_in_bytes, actual_size_of_scan_in_bytes);


    harness.destroy();
    Ok(())
}



#[test]
pub fn directory_scan_follows_file_and_directory_symlink_when_configured() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let simple_harness = SimpleTree::initialize();


    let symlinked_file_original_path = {
        let symlink_path = deep_harness.child_path("symlink.file");
        symlink_path.assert_not_exists();
        symlink_path.symlink_to_file(simple_harness.empty_txt.as_path());

        symlink_path
    };

    let symlinked_dir_original_path = {
        let symlink_path = deep_harness.child_path("symlink.dir");
        symlink_path.assert_not_exists();
        symlink_path.symlink_to_directory(simple_harness.yes.as_path());

        symlink_path
    };


    let scan_results = fs_more::directory::DirectoryScan::scan_with_options(
        deep_harness.as_path(),
        DirectoryScanDepthLimit::Unlimited,
        false,
    )
    .unwrap();


    assert_path_list_fully_matches_set(
        scan_results.files(),
        [
            deep_harness.a_bin.as_path(),
            deep_harness.foo.b_bin.as_path(),
            deep_harness.foo.bar.c_bin.as_path(),
            deep_harness.foo.bar.hello.world.d_bin.as_path(),
            symlinked_file_original_path.as_path(),
        ],
    );

    assert_path_list_fully_matches_set(
        scan_results.directories(),
        [
            deep_harness.foo.as_path(),
            deep_harness.foo.bar.as_path(),
            deep_harness.foo.bar.hello.as_path(),
            deep_harness.foo.bar.hello.world.as_path(),
            symlinked_dir_original_path.as_path(),
        ],
    );


    deep_harness.destroy();
    simple_harness.destroy();
    Ok(())
}

#[test]
pub fn directory_scan_does_not_follow_file_and_directory_symlink_when_configured() -> TestResult {
    let deep_harness = DeepTree::initialize();
    let simple_harness = SimpleTree::initialize();


    {
        let symlink_path = deep_harness.child_path("symlink.file");
        symlink_path.assert_not_exists();
        symlink_path.symlink_to_file(simple_harness.empty_txt.as_path());
    }

    {
        let symlink_path = deep_harness.child_path("symlink.dir");
        symlink_path.assert_not_exists();
        symlink_path.symlink_to_directory(simple_harness.yes.as_path());
    }


    let scan_results = fs_more::directory::DirectoryScan::scan_with_options(
        deep_harness.as_path(),
        DirectoryScanDepthLimit::Unlimited,
        true,
    )
    .unwrap();


    assert_path_list_fully_matches_set(
        scan_results.files(),
        [
            deep_harness.a_bin.as_path(),
            deep_harness.foo.b_bin.as_path(),
            deep_harness.foo.bar.c_bin.as_path(),
            deep_harness.foo.bar.hello.world.d_bin.as_path(),
            simple_harness.empty_txt.as_path(),
            simple_harness.yes.hello_world_txt.as_path(),
            simple_harness.yes.no_bin.as_path(),
        ],
    );

    assert_path_list_fully_matches_set(
        scan_results.directories(),
        [
            deep_harness.foo.as_path(),
            deep_harness.foo.bar.as_path(),
            deep_harness.foo.bar.hello.as_path(),
            deep_harness.foo.bar.hello.world.as_path(),
            simple_harness.yes.as_path(),
        ],
    );


    deep_harness.destroy();
    simple_harness.destroy();
    Ok(())
}


#[test]
pub fn directory_size_in_bytes_produces_correct_information() -> TestResult {
    let harness = SimpleTree::initialize();


    let actual_size_of_harness_in_bytes = {
        let empty_txt_size_bytes = harness.empty_txt.size_in_bytes();
        let foo_dir_size_bytes = harness.yes.size_in_bytes();
        let hello_world_size_bytes = harness.yes.hello_world_txt.size_in_bytes();
        let bar_bin_size_bytes = harness.yes.no_bin.size_in_bytes();

        empty_txt_size_bytes + foo_dir_size_bytes + hello_world_size_bytes + bar_bin_size_bytes
    };


    let directory_size_bytes =
        fs_more::directory::directory_size_in_bytes(harness.as_path(), false).unwrap();

    assert_eq!(directory_size_bytes, actual_size_of_harness_in_bytes);


    harness.destroy();
    Ok(())
}
