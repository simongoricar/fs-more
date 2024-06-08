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

impl FileState {
    pub fn capture_from_file_path<P>(file_path: P) -> Self
    where
        P: AsRef<Path>,
    {
        if !file_path
            .as_ref()
            .try_exists()
            .expect("failed to read file metadata")
        {
            return Self::NonExistent;
        }

        if !file_path.as_ref().is_file() {
            panic!(
                "expected the provided path \"{}\" to lead to a file",
                file_path.as_ref().display()
            );
        }


        let file_contents = fs::read(file_path.as_ref()).expect("failed to read file contents");

        if file_contents.is_empty() {
            return Self::Empty;
        }

        Self::NonEmpty {
            content: file_contents,
        }
    }

    #[inline]
    pub fn equals_other_file_state(&self, other: &Self) -> bool {
        self == other
    }
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
        Self {
            file_path: path.as_ref().to_path_buf(),
            captured_state: FileState::capture_from_file_path(path),
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

    pub fn assert_captured_states_equal(&self, other: &Self) {
        assert!(
            self.captured_state
                .equals_other_file_state(&other.captured_state),
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
        self.assert_captured_states_equal(&captured_other_file);
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