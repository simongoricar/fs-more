use fs_more_test_harness::{
    assertable::AsPath,
    prelude::FileSystemHarness,
    trees::structures::{empty::EmptyTree, simple::SimpleTree},
};


#[test]
fn is_directory_empty_reports_true_on_empty_directory() {
    let empty_harness = EmptyTree::initialize();

    let is_empty = fs_more::directory::is_directory_empty(empty_harness.as_path()).unwrap();

    assert!(is_empty);
}


#[test]
fn is_directory_empty_reports_false_on_non_empty_directory() {
    let simple_harness = SimpleTree::initialize();

    let is_empty = fs_more::directory::is_directory_empty(simple_harness.as_path()).unwrap();

    assert!(!is_empty);
}
