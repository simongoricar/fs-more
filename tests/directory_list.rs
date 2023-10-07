use std::path::Path;

use fs_more_test_harness::{error::TestResult, trees::SimpleTreeHarness};

/// Returns `true` if the provided `Vec` of `AsRef<Path>`-implementing items
/// contains at least one path matching the `target_path`.
fn path_vec_contains_path<I, P, R>(path_vec: I, target_path: R) -> bool
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    R: AsRef<Path>,
{
    path_vec
        .into_iter()
        .any(|path| path.as_ref().eq(target_path.as_ref()))
}

#[test]
pub fn scan_directory() -> TestResult<()> {
    let harness = SimpleTreeHarness::new()?;

    let scan_result =
        fs_more::directory::DirectoryScan::scan_with_options(harness.root.path(), None, false);

    assert!(
        scan_result.is_ok(),
        "DirectoryScan::scan_with_options did not return Ok, but {}",
        scan_result.unwrap_err(),
    );
    let scan = scan_result.unwrap();

    assert_eq!(
        scan.directories.len(),
        1,
        "Unexpected amount of scanned directories."
    );
    assert_eq!(
        scan.files.len(),
        2,
        "Unexpected amount of scanned files."
    );

    assert!(!scan.is_real_directory_deeper_than_scan);


    assert!(path_vec_contains_path(
        &scan.files,
        harness.binary_file_a.path()
    ));
    assert!(path_vec_contains_path(
        &scan.files,
        harness.binary_file_b.path()
    ));


    assert!(!path_vec_contains_path(
        &scan.directories,
        harness.root.path(),
    ));
    assert!(path_vec_contains_path(
        &scan.directories,
        harness.subdirectory_b.path(),
    ));

    harness.destroy()?;
    Ok(())
}


#[test]
pub fn scan_directory_with_limited_depth() -> TestResult<()> {
    let harness = SimpleTreeHarness::new()?;

    let scan_result =
        fs_more::directory::DirectoryScan::scan_with_options(harness.root.path(), Some(0), false);

    assert!(
        scan_result.is_ok(),
        "DirectoryScan::scan_with_options did not return Ok, but {}",
        scan_result.unwrap_err(),
    );
    let scan = scan_result.unwrap();

    assert_eq!(
        scan.directories.len(),
        1,
        "Unexpected amount of scanned directories."
    );
    assert_eq!(
        scan.files.len(),
        1,
        "Unexpected amount of scanned files."
    );

    assert!(scan.is_real_directory_deeper_than_scan);


    assert!(path_vec_contains_path(
        &scan.files,
        harness.binary_file_a.path()
    ));
    assert!(!path_vec_contains_path(
        &scan.files,
        harness.binary_file_b.path()
    ));

    assert!(!path_vec_contains_path(
        &scan.directories,
        harness.root.path(),
    ));
    assert!(path_vec_contains_path(
        &scan.directories,
        harness.subdirectory_b.path(),
    ));


    harness.destroy()?;
    Ok(())
}


#[test]
pub fn directory_size_via_directory_scan() -> TestResult<()> {
    let harness = SimpleTreeHarness::new()?;

    let actual_size_in_bytes = harness.binary_file_a.path().metadata().unwrap().len()
        + harness.binary_file_b.path().metadata().unwrap().len()
        + harness.subdirectory_b.path().metadata().unwrap().len();


    let scan_result =
        fs_more::directory::DirectoryScan::scan_with_options(harness.root.path(), None, false);


    assert!(
        scan_result.is_ok(),
        "DirectoryScan::scan_with_options did not return Ok, but {}",
        scan_result.unwrap_err(),
    );
    let scan = scan_result.unwrap();

    assert!(!scan.is_real_directory_deeper_than_scan);

    let size_in_bytes = scan
        .total_size_in_bytes()
        .expect("Failed to calculate size of scan.");

    // One 32 KiB file, one 64 KiB file.
    assert_eq!(
        size_in_bytes, actual_size_in_bytes,
        "Unexpected total size in bytes (expected one 32 KiB and one 64 KiB file)"
    );


    harness.destroy()?;
    Ok(())
}

#[test]
pub fn directory_size_via_directory_scan_with_depth_limit() -> TestResult<()> {
    let harness = SimpleTreeHarness::new()?;

    let actual_size_in_bytes = harness.binary_file_a.path().metadata().unwrap().len()
        + harness.subdirectory_b.path().metadata().unwrap().len();


    let scan_result =
        fs_more::directory::DirectoryScan::scan_with_options(harness.root.path(), Some(0), false);


    assert!(
        scan_result.is_ok(),
        "DirectoryScan::scan_with_options did not return Ok, but {}",
        scan_result.unwrap_err(),
    );
    let scan = scan_result.unwrap();

    assert!(scan.is_real_directory_deeper_than_scan);

    let size_in_bytes = scan
        .total_size_in_bytes()
        .expect("Failed to calculate size of scan.");

    // Just one 32 KiB file.
    assert_eq!(
        size_in_bytes, actual_size_in_bytes,
        "Unexpected total size in bytes (expected one 32 KiB file in depth-limited scan)"
    );


    harness.destroy()?;
    Ok(())
}



#[test]
pub fn directory_size_via_size_function() -> TestResult<()> {
    let harness = SimpleTreeHarness::new()?;

    let actual_size_in_bytes = harness.binary_file_a.path().metadata().unwrap().len()
        + harness.binary_file_b.path().metadata().unwrap().len()
        + harness.subdirectory_b.path().metadata().unwrap().len();


    let size_in_bytes_result =
        fs_more::directory::directory_size_in_bytes(harness.root.path(), false);


    assert!(
        size_in_bytes_result.is_ok(),
        "get_directory_size did not return Ok, but {}",
        size_in_bytes_result.unwrap_err(),
    );
    let size_in_bytes = size_in_bytes_result.unwrap();


    // One 32 KiB file, one 64 KiB file.
    assert_eq!(
        size_in_bytes, actual_size_in_bytes,
        "Unexpected total size in bytes (expected one 32 KiB and one 64 KiB file)"
    );


    harness.destroy()?;
    Ok(())
}
