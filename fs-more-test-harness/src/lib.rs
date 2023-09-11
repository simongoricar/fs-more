pub mod double;
pub mod error;
pub mod single;

pub use double::DoubleFileHarness;
pub use single::SingleFileHarness;

#[macro_export]
macro_rules! assert_file_content_match {
    ( $file_path:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            std::fs::read_to_string($file_path).unwrap(),
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
