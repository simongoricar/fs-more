use std::{
    fs::{self, OpenOptions},
    io::prelude::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FileState {
    NonExistent,
    Empty,
    NonEmpty { content: Vec<u8> },
}

pub struct CapturedFileState {
    file_path: PathBuf,

    captured_state: FileState,
}

impl CapturedFileState {
    pub fn new_with_content_capture<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        if !path
            .as_ref()
            .try_exists()
            .expect("failed to read file metadata")
        {
            return Self {
                file_path: path.as_ref().to_path_buf(),
                captured_state: FileState::NonExistent,
            };
        }

        if !path.as_ref().is_file() {
            panic!(
                "expected the provided path \"{}\" to lead to a file",
                path.as_ref().display()
            );
        }


        let file_contents = fs::read(path.as_ref()).expect("failed to read file contents");

        if file_contents.is_empty() {
            return Self {
                file_path: path.as_ref().to_path_buf(),
                captured_state: FileState::Empty,
            };
        }

        Self {
            file_path: path.as_ref().to_path_buf(),
            captured_state: FileState::NonEmpty {
                content: file_contents,
            },
        }
    }

    pub fn new_with_manual_state<P>(path: P, state: FileState) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            file_path: path.as_ref().to_path_buf(),
            captured_state: state,
        }
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    pub fn assert_captured_state_equals_other(&self, other: &Self) {
        assert_eq!(
            self.captured_state,
            other.captured_state,
            "files \"{}\" and \"{}\" don't have equal states",
            self.file_path.display(),
            other.file_path.display(),
        )
    }

    pub fn assert_captured_state_matches_other_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let captured_other_file = Self::new_with_content_capture(other);
        self.assert_captured_state_equals_other(&captured_other_file);
    }

    pub fn assert_unchanged(&self) {
        let now_exists = self
            .file_path
            .try_exists()
            .expect("failed to read file metadata");

        match &self.captured_state {
            FileState::NonExistent => {
                if now_exists {
                    panic!(
                        "captured state is NonExistent, but file \"{}\" exists",
                        self.file_path.display()
                    );
                }
            }
            FileState::Empty => {
                if !now_exists {
                    panic!(
                        "captured state is Empty, but file \"{}\" does not exist",
                        self.file_path.display()
                    );
                }

                let file = OpenOptions::new()
                    .read(true)
                    .open(&self.file_path)
                    .expect("failed to open file");

                if file.bytes().next().is_some() {
                    panic!(
                        "captured state is Empty, but file \"{}\" is not empty",
                        self.file_path.display()
                    );
                }
            }
            FileState::NonEmpty { content } => {
                if !now_exists {
                    panic!(
                        "captured state is NonEmpty, but file \"{}\" does not exist",
                        self.file_path.display()
                    );
                }

                let fresh_file_contents =
                    fs::read(&self.file_path).expect("failed to read file contents");

                assert_eq!(
                    content,
                    &fresh_file_contents,
                    "captured state is NonEmpty, but file \"{}\" does not match the captured content",
                    self.file_path.display()
                );
            }
        }
    }
}
