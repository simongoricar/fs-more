use fs_more::directory::{
    DirectoryCopyOptions,
    DirectoryCopyProgress,
    DirectoryCopyWithProgressOptions,
    DirectoryScan,
};
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


#[test]
pub fn copy_directory_with_progress() -> TestResult<()> {
    let harness = DeepTreeHarness::new()?;
    let empty_harness = EmptyTreeHarness::new()?;

    let source_scan =
        DirectoryScan::scan_with_options(harness.root.path(), None, false)
            .expect("failed to scan temporary directory");
    let source_full_size = source_scan
        .total_size_in_bytes()
        .expect("failed to compute size of source directory in bytes");

    empty_harness.root.assert_is_empty();

    let mut last_progress: Option<DirectoryCopyProgress> = None;

    // TODO
    let finished_copy = fs_more::directory::copy_directory_with_progress(
        harness.root.path(),
        empty_harness.root.path(),
        DirectoryCopyWithProgressOptions {
            allow_existing_target_directory: true,
            ..Default::default()
        },
        |progress| last_progress = Some(progress.clone()),
    )
    .unwrap_or_else(|error| {
        panic!(
            "copy_directory_with_progress unexpectedly failed with Err: {}",
            error
        );
    });


    assert!(
        last_progress.is_some(),
        "copy_directory_with_progress did not report progress at all"
    );

    let last_progress = last_progress.unwrap();

    assert_eq!(
        last_progress.bytes_finished,
        last_progress.bytes_total,
        "copy_directory_with_progress's last progress message was an unfinished copy"
    );
    assert_eq!(
        source_full_size,
        last_progress.bytes_total,
        "DirectoryScan and copy_directory_with_progress's last progress reported different amount of total bytes"
    );
    assert_eq!(
        source_full_size, finished_copy.total_bytes_copied,
        "DirectoryScan and copy_directory_with_progress report different amount of total bytes"
    );

    assert_eq!(
        source_scan.files.len(),
        last_progress.files_copied,
        "copy_directory_with_progress's last progress did not report all files"
    );
    assert_eq!(
        source_scan.files.len(),
        finished_copy.num_files_copied,
        "DirectoryScan and copy_directory_with_progress report different number of files"
    );

    assert_eq!(
        source_scan.directories.len(),
        last_progress.directories_created,
        "copy_directory_with_progress's last progress did not report all directories"
    );
    assert_eq!(
        source_scan.directories.len(),
        finished_copy.num_directories_created,
        "DirectoryScan and copy_directory_with_progress report different number of directories"
    );

    harness
        .root
        .assert_directory_contents_match_directory(empty_harness.root.path());


    harness.destroy()?;
    empty_harness.destroy()?;
    Ok(())
}
