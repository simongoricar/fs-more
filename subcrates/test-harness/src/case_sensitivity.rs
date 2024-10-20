use std::fs::{File, OpenOptions};

use crate::assertable::AssertablePath;


/// Returns `true` if the temporary directory on the current file system
/// is case-sensitive.
///
/// Internally, this create a temporary directory, creates a file,
/// and attempts to deduce case-sensitivity from a differently-cased read.
///
/// # Panics
/// Panics on any IO error (when failing to create a temporary directory, creating a file, ...).
/// This is okay in our case, because this function is part of the test harness.
pub fn detect_case_sensitivity_for_temp_dir() -> bool {
    let temporary_directory = tempfile::tempdir().expect("failed to create temporary directory");


    let hello_txt_path = temporary_directory.path().join("HELLO.txt");
    let hello_txt_path_lowercase = temporary_directory.path().join("hello.txt");

    hello_txt_path.assert_not_exists();
    hello_txt_path_lowercase.assert_not_exists();

    OpenOptions::new()
        .create_new(true)
        .open(&hello_txt_path)
        .expect("failed to create HELLO.txt");

    hello_txt_path.assert_is_file_and_not_symlink();


    if !hello_txt_path_lowercase
        .try_exists()
        .expect("failed to even try to open hello.txt")
    {
        // Filesystem is case-sensitive (HELLO.txt exists, but trying to open hello.txt fails).

        temporary_directory
            .close()
            .expect("failed to close temporary directory");

        true
    } else {
        // Filesystem is case-insensitive (HELLO.txt exists, and opening hello.txt works).

        temporary_directory
            .close()
            .expect("failed to close temporary directory");

        false
    }
}
