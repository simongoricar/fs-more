use fs_more::directory::{DirectoryCopyOptions, DirectoryScan};
use fs_more_test_harness::{
    error::TestResult,
    trees::{DeepTreeHarness, EmptyTreeHarness},
};

#[test]
pub fn copy_directory() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan =
        DirectoryScan::scan_with_options(harness.root.path(), None, false)
            .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let finished_copy = fs_more::directory::copy_directory(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory unexpectedly failed with Err: {}",
            error
        );
    });


    assert_eq!(
        source_full_size, finished_copy.total_bytes_copied,
        "DirectoryScan and copy_directory report different amount of bytes"
    );

    assert_eq!(
        source_scan.files.len(),
        finished_copy.num_files_copied,
        "DirectoryScan and copy_directory report different number of files"
    );

    assert_eq!(
        source_scan.directories.len(),
        finished_copy.num_directories_created,
        "DirectoryScan and copy_directory report different number of directories"
    );

    harness
        .root
        .assert_directory_contents_match_directory(empty_harness.root.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}
