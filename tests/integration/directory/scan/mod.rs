use fs_more::{
    directory::{DirectoryScanDepthLimit, DirectoryScanOptions, DirectoryScanner},
    error::DirectoryScanError,
};
use fs_more_test_harness::{
    assert_path_list_fully_matches_set,
    assert_path_list_fully_matches_with_counted_ocucrrences,
    prelude::*,
    trees::structures::{
        deep::DeepTree,
        empty::EmptyTree,
        simple::SimpleTree,
        symlink_cycle::SymlinkCycleTree,
        symlinked::SymlinkedTree,
    },
};



#[test]
fn scanner_iter_produces_all_expected_paths() {
    let simple_tree = SimpleTree::initialize();

    let scanner = DirectoryScanner::new(
        simple_tree.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_set(
        scanned_paths,
        [
            simple_tree.as_path(),
            simple_tree.empty_txt.as_path(),
            simple_tree.yes.as_path(),
            simple_tree.yes.hello_world_txt.as_path(),
            simple_tree.yes.no_bin.as_path(),
        ],
    );


    simple_tree.destroy();



    let deep_tree = DeepTree::initialize();

    let scanner = DirectoryScanner::new(
        deep_tree.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_set(
        scanned_paths,
        [
            deep_tree.as_path(),
            deep_tree.a_bin.as_path(),
            deep_tree.foo.as_path(),
            deep_tree.foo.b_bin.as_path(),
            deep_tree.foo.bar.as_path(),
            deep_tree.foo.bar.c_bin.as_path(),
            deep_tree.foo.bar.hello.as_path(),
            deep_tree.foo.bar.hello.world.as_path(),
            deep_tree.foo.bar.hello.world.d_bin.as_path(),
        ],
    );


    deep_tree.destroy();
}



#[test]
fn scanner_iter_skips_base_directory_if_configured() {
    let simple_tree = SimpleTree::initialize();

    let scanner = DirectoryScanner::new(
        simple_tree.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            yield_base_directory: false,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_set(
        scanned_paths,
        [
            simple_tree.empty_txt.as_path(),
            simple_tree.yes.as_path(),
            simple_tree.yes.hello_world_txt.as_path(),
            simple_tree.yes.no_bin.as_path(),
        ],
    );


    simple_tree.destroy();
}


#[test]
fn scanner_iter_respects_depth_limit() {
    let deep_harness = DeepTree::initialize();


    {
        let scanner = DirectoryScanner::new(
            deep_harness.as_path(),
            DirectoryScanOptions {
                maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
                ..Default::default()
            },
        )
        .into_iter();


        let scanned_paths = scanner
            .map(|entry_result| entry_result.map(|entry| entry.into_path()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_path_list_fully_matches_set(
            scanned_paths,
            [
                deep_harness.as_path(),
                deep_harness.a_bin.as_path(),
                deep_harness.foo.as_path(),
                deep_harness.foo.b_bin.as_path(),
                deep_harness.foo.bar.as_path(),
                deep_harness.foo.bar.c_bin.as_path(),
                deep_harness.foo.bar.hello.as_path(),
                deep_harness.foo.bar.hello.world.as_path(),
                deep_harness.foo.bar.hello.world.d_bin.as_path(),
            ],
        );
    }


    {
        let scanner = DirectoryScanner::new(
            deep_harness.as_path(),
            DirectoryScanOptions {
                maximum_scan_depth: DirectoryScanDepthLimit::Limited { maximum_depth: 0 },
                ..Default::default()
            },
        )
        .into_iter();


        let scanned_paths = scanner
            .map(|entry_result| entry_result.map(|entry| entry.into_path()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_path_list_fully_matches_set(
            scanned_paths,
            [
                deep_harness.as_path(),
                deep_harness.a_bin.as_path(),
                deep_harness.foo.as_path(),
            ],
        );
    }

    {
        let scanner = DirectoryScanner::new(
            deep_harness.as_path(),
            DirectoryScanOptions {
                maximum_scan_depth: DirectoryScanDepthLimit::Limited { maximum_depth: 2 },
                ..Default::default()
            },
        )
        .into_iter();


        let scanned_paths = scanner
            .map(|entry_result| entry_result.map(|entry| entry.into_path()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_path_list_fully_matches_set(
            scanned_paths,
            [
                deep_harness.as_path(),
                deep_harness.a_bin.as_path(),
                deep_harness.foo.as_path(),
                deep_harness.foo.b_bin.as_path(),
                deep_harness.foo.bar.as_path(),
                deep_harness.foo.bar.c_bin.as_path(),
                deep_harness.foo.bar.hello.as_path(),
            ],
        );
    }


    deep_harness.destroy();
}



#[test]
fn scanner_iter_sums_into_correct_size() {
    let deep_harness = DeepTree::initialize();

    let actual_size_of_harness_in_bytes = {
        let root_size = deep_harness.size_in_bytes();
        let a_bin_size = deep_harness.a_bin.size_in_bytes();
        let foo_size = deep_harness.foo.size_in_bytes();
        let b_bin_size = deep_harness.foo.b_bin.size_in_bytes();
        let bar_size = deep_harness.foo.bar.size_in_bytes();
        let c_bin_size = deep_harness.foo.bar.c_bin.size_in_bytes();
        let hello_size = deep_harness.foo.bar.hello.size_in_bytes();
        let world_size = deep_harness.foo.bar.hello.world.size_in_bytes();
        let d_bin_size = deep_harness.foo.bar.hello.world.d_bin.size_in_bytes();

        root_size
            + a_bin_size
            + foo_size
            + b_bin_size
            + bar_size
            + c_bin_size
            + hello_size
            + world_size
            + d_bin_size
    };


    let scanner = DirectoryScanner::new(
        deep_harness.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_directory_size = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_metadata().len()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .sum::<u64>();


    assert_eq!(scanned_directory_size, actual_size_of_harness_in_bytes);


    deep_harness.destroy();
}



#[test]
fn scanner_iter_follows_symlinks_if_enabled() {
    let tree_harness = SymlinkedTree::initialize();


    let scanner = DirectoryScanner::new(
        tree_harness.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: true,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_with_counted_ocucrrences(
        scanned_paths,
        [
            (tree_harness.as_path(), 1),
            (tree_harness.a_bin.as_path(), 1),
            (tree_harness.foo.as_path(), 1),
            (tree_harness.foo.bar.as_path(), 1),
            (tree_harness.foo.bar.hello.as_path(), 2),
            (tree_harness.foo.bar.c_bin.as_path(), 1),
            (tree_harness.foo.bar.hello.world.as_path(), 2),
            (tree_harness.foo.bar.hello.world.d_bin.as_path(), 3),
            (tree_harness.foo.b_bin.as_path(), 1),
        ],
    );

    tree_harness.destroy();
}



#[test]
fn scanner_iter_does_not_follow_symlinks_if_not_enabled() {
    let tree_harness = SymlinkedTree::initialize();


    let scanner = DirectoryScanner::new(
        tree_harness.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: false,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_with_counted_ocucrrences(
        scanned_paths,
        [
            (tree_harness.as_path(), 1),
            (tree_harness.a_bin.as_path(), 1),
            (tree_harness.foo.as_path(), 1),
            (tree_harness.foo.b_bin.as_path(), 1),
            (tree_harness.foo.symlink_to_hello.as_path(), 1),
            (tree_harness.foo.symlink_to_d_bin.as_path(), 1),
            (tree_harness.foo.bar.as_path(), 1),
            (tree_harness.foo.bar.c_bin.as_path(), 1),
            (tree_harness.foo.bar.hello.as_path(), 1),
            (tree_harness.foo.bar.hello.world.as_path(), 1),
            (tree_harness.foo.bar.hello.world.d_bin.as_path(), 1),
        ],
    );

    tree_harness.destroy();
}


#[test]
fn scanner_iter_follows_base_scan_directory_symlink_if_enabled() {
    let scan_source_harness = EmptyTree::initialize();
    let tree_harness = SymlinkedTree::initialize();


    let symlink_to_tree = scan_source_harness.child_path("symlink-to-tree");
    symlink_to_tree.assert_not_exists();
    symlink_to_tree.symlink_to_directory(tree_harness.as_path());


    let scanner = DirectoryScanner::new(
        &symlink_to_tree,
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_base_directory_symbolic_link: true,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_with_counted_ocucrrences(
        scanned_paths,
        [
            (tree_harness.as_path(), 1),
            (tree_harness.a_bin.as_path(), 1),
            (tree_harness.foo.as_path(), 1),
            (tree_harness.foo.b_bin.as_path(), 1),
            (tree_harness.foo.symlink_to_hello.as_path(), 1),
            (tree_harness.foo.symlink_to_d_bin.as_path(), 1),
            (tree_harness.foo.bar.as_path(), 1),
            (tree_harness.foo.bar.c_bin.as_path(), 1),
            (tree_harness.foo.bar.hello.as_path(), 1),
            (tree_harness.foo.bar.hello.world.as_path(), 1),
            (tree_harness.foo.bar.hello.world.d_bin.as_path(), 1),
        ],
    );


    tree_harness.destroy();
    scan_source_harness.destroy();
}


#[test]
fn scanner_iter_does_not_follow_base_scan_directory_symlink_if_not_enabled() {
    let scan_source_harness = EmptyTree::initialize();
    let tree_harness = SymlinkedTree::initialize();


    let symlink_to_tree = scan_source_harness.child_path("symlink-to-tree");
    symlink_to_tree.assert_not_exists();
    symlink_to_tree.symlink_to_directory(tree_harness.as_path());


    let scanner = DirectoryScanner::new(
        &symlink_to_tree,
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_base_directory_symbolic_link: false,
            ..Default::default()
        },
    )
    .into_iter();


    let scanned_paths = scanner
        .map(|entry_result| entry_result.map(|entry| entry.into_path()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_path_list_fully_matches_with_counted_ocucrrences(
        scanned_paths,
        [(symlink_to_tree.as_path(), 1)],
    );


    tree_harness.destroy();
    scan_source_harness.destroy();
}



#[test]
fn scanner_iter_errors_on_symbolic_link_cycle() {
    let cyclical_tree = SymlinkCycleTree::initialize();


    let scanner = DirectoryScanner::new(
        cyclical_tree.as_path(),
        DirectoryScanOptions {
            maximum_scan_depth: DirectoryScanDepthLimit::Unlimited,
            follow_symbolic_links: true,
            ..Default::default()
        },
    );


    let mut symlink_cycle_detected = false;

    for entry in scanner {
        match entry {
            Ok(_) => {}
            Err(error) => {
                if let DirectoryScanError::SymlinkCycleEncountered { directory_path } = error {
                    if directory_path != cyclical_tree.foo.as_path() {
                        panic!(
                            "got symlink cycle detection, but incorrect path: {}",
                            directory_path.display()
                        );
                    } else if symlink_cycle_detected {
                        panic!("got more than one symlink cycle detection");
                    } else {
                        symlink_cycle_detected = true;
                    }
                } else {
                    panic!("unexpected error: {}", error);
                }
            }
        }
    }

    assert!(symlink_cycle_detected);


    cyclical_tree.destroy();
}
