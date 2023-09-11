use std::path::Path;

use assert_fs::{
    fixture::{ChildPath, FixtureError},
    prelude::{FileWriteStr, PathChild},
    TempDir,
};

const SINGLE_FILE_CONTENTS: &str = "This is the first file.";

pub struct SingleFileHarness {
    temp_dir: TempDir,
    pub single_file: ChildPath,
}

impl SingleFileHarness {
    pub fn new() -> Result<Self, FixtureError> {
        let temp_dir = TempDir::new()?;

        let single_file = temp_dir.child("test_file.txt");
        single_file.write_str(SINGLE_FILE_CONTENTS)?;

        assert_eq!(
            std::fs::read_to_string(single_file.path()).unwrap(),
            SINGLE_FILE_CONTENTS,
            "SingleFileHarness setup failed.",
        );

        Ok(Self {
            temp_dir,
            single_file,
        })
    }

    pub fn file_path(&self) -> &Path {
        self.single_file.path()
    }

    pub fn expected_file_contents() -> &'static str {
        SINGLE_FILE_CONTENTS
    }

    pub fn destroy(self) -> Result<(), FixtureError> {
        self.temp_dir.close()
    }
}

/*
#[macro_export]
macro_rules! assert_single_file_content_match {
    ( $single_harness:expr ) => {
        assert_eq!(
            std::fs::read_to_string($single_harness.single_file.path()).unwrap(),
            $crate::single::SingleFileHarness::expected_file_contents(),
            "File contents do not match the expected value.",
        );
    };

    ( $single_harness:expr, $explicit_expected_content:expr ) => {
        assert_eq!(
            std::fs::read_to_string($single_harness.single_file.path()).unwrap(),
            $explicit_expected_content,
            "File contents do not match the expected value.",
        );
    };

    ( $single_harness:expr, $explicit_expected_content:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($single_harness.single_file.path()).unwrap(),
            $explicit_expected_content,
            $($arg)+,
        );
    };

    ( $single_harness:expr, otherwise $($arg:tt)+ ) => {
        assert_eq!(
            std::fs::read_to_string($single_harness.single_file.path()).unwrap(),
            $crate::single::SingleFileHarness::expected_file_contents(),
            $($arg)+,
        );
    };
} */
