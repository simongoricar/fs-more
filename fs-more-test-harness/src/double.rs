use std::path::Path;

use assert_fs::{
    fixture::{ChildPath, FixtureError},
    prelude::{FileWriteStr, PathChild},
    TempDir,
};

pub const FIRST_FILE_NAME: &str = "test_file1.txt";
const FIRST_FILE_CONTENTS: &str = "This is the first file.";

pub const SECOND_FILE_NAME: &str = "test_file2.txt";
const SECOND_FILE_CONTENTS: &str = "This is the second file.";


pub struct DoubleFileHarness {
    temp_dir: TempDir,
    pub first_file: ChildPath,
    pub second_file: ChildPath,
}

impl DoubleFileHarness {
    pub fn new() -> Result<Self, FixtureError> {
        let temp_dir = TempDir::new()?;

        let first_file = temp_dir.child(FIRST_FILE_NAME);
        first_file.write_str(FIRST_FILE_CONTENTS)?;

        let second_file = temp_dir.child(SECOND_FILE_NAME);
        second_file.write_str(SECOND_FILE_CONTENTS)?;

        assert_eq!(
            std::fs::read_to_string(first_file.path()).unwrap(),
            FIRST_FILE_CONTENTS,
            "DoubleFileHarness setup failed (first file content mismatch).",
        );

        assert_eq!(
            std::fs::read_to_string(second_file.path()).unwrap(),
            SECOND_FILE_CONTENTS,
            "DoubleFileHarness setup failed (second file content mismatch).",
        );

        Ok(Self {
            temp_dir,
            first_file,
            second_file,
        })
    }

    pub fn first_file_path(&self) -> &Path {
        self.first_file.path()
    }

    pub fn second_file_path(&self) -> &Path {
        self.second_file.path()
    }

    pub fn expected_first_file_contents() -> &'static str {
        FIRST_FILE_CONTENTS
    }

    pub fn expected_second_file_contents() -> &'static str {
        SECOND_FILE_CONTENTS
    }

    pub fn destroy(self) -> Result<(), FixtureError> {
        self.temp_dir.close()
    }
}

/*
#[macro_export]
macro_rules! assert_first_file_content_match {
    ( $double_harness:expr ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.first_file.path()).unwrap(),
            $crate::double::DoubleFileHarness::expected_first_file_contents(),
            "File contents do not match the expected value.",
        );
    };

    ( $double_harness:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.first_file.path()).unwrap(),
            $explicit_expected_content,
            "File contents do not match the expected value.",
        );
    };

    ( $double_harness:expr, $explicit_expected_content:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.first_file.path()).unwrap(),
            $explicit_expected_content,
            $($arg)+,
        );
    };

    ( $double_harness:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.first_file.path()).unwrap(),
            $crate::double::DoubleFileHarness::expected_first_file_contents(),
            $($arg)+,
        );
    };
}

#[macro_export]
macro_rules! assert_second_file_content_match {
    ( $double_harness:expr ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.second_file.path()).unwrap(),
            $crate::double::DoubleFileHarness::expected_second_file_contents(),
            "File contents do not match the expected value.",
        );
    };

    ( $double_harness:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.second_file.path()).unwrap(),
            $explicit_expected_content,
            "File contents do not match the expected value.",
        );
    };

    ( $double_harness:expr, $explicit_expected_content:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.second_file.path()).unwrap(),
            $explicit_expected_content,
            $($arg)+,
        );
    };

    ( $double_harness:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($double_harness.second_file.path()).unwrap(),
            $crate::double::DoubleFileHarness::expected_second_file_contents(),
            $($arg)+,
        );
    };
}
 */
