/// This macro reads the file at the given path *as a string*
/// and uses `assert_eq` to compare the contents with the expected ones (which can be of `String` or `&str` type).
///
/// If you need a byte-by-byte comparison, see [`assert_file_bytes_match`][crate::assert_file_bytes_match] instead.
///
/// ## Example
/// ```no_run
/// # use fs_more_test_harness::assert_file_string_match;
/// # use std::path::Path;
///
/// assert_file_string_match!(
///     Path::new("/some/path/to/a/file.txt"),
///     String::from("The expected contents of the file."),
///     otherwise "your assertion error here"
/// );
/// ```
///
/// You may also skip the last parameter:
/// ```no_run
/// # use fs_more_test_harness::assert_file_string_match;
/// # use std::path::Path;
///
/// assert_file_string_match!(
///     Path::new("/some/path/to/a/file.txt"),
///     "The expected contents of the file."
/// );
/// ```
#[macro_export]
macro_rules! assert_file_string_match {
    ( $file_path:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            &std::fs::read_to_string($file_path).unwrap(),
            $explicit_expected_content,
            "File contents do not match the expected value.",
        );
    };

    ( $file_path:expr, $explicit_expected_content:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($file_path).unwrap(),
            $explicit_expected_content,
            $($arg)+,
        );
    };
}

/// This macro reads the file at the given path *as a `Vec` of `u8` values*
/// and uses `assert_eq` to compare the contents with the expected ones.
///
/// If you need a string comparison, see [`assert_file_string_match`][crate::assert_file_string_match] instead.
///
/// ## Example
/// ```no_run
/// # use fs_more_test_harness::assert_file_bytes_match;
/// # use std::path::Path;
///
/// assert_file_bytes_match!(
///     Path::new("/some/path/to/a/file.txt"),
///     [12u8, 0u8],
///     otherwise "your assertion error here"
/// );
/// ```
///
/// You may also skip the last parameter:
/// ```no_run
/// # use fs_more_test_harness::assert_file_bytes_match;
/// # use std::path::Path;
///
/// assert_file_bytes_match!(
///     Path::new("/some/path/to/a/file.txt"),
///     &vec![12u8, 0u8]
/// );
/// ```
#[macro_export]
macro_rules! assert_file_bytes_match {
    ( $file_path:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            &std::fs::read($file_path).unwrap(),
            $explicit_expected_content,
            "File contents do not match the expected value.",
        );
    };

    ( $file_path:expr, $explicit_expected_content:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read($file_path).unwrap(),
            $explicit_expected_content,
            $($arg)+,
        );
    };
}
