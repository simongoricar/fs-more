use std::{
    fmt::Debug,
    fs::{self, OpenOptions},
    io::prelude::Read,
    path::{Path, PathBuf},
};


/// Formats the given byte `slice` up to the specified `maximum_displayed_length`.
///
/// If the slice is longer than the maximum length, the "overflowing" bytes are not displayed.
///
/// ```ignore
/// assert_eq!(
///     abbreviate_u8_slice_as_bytes(&[1, 2, 3, 4], 2),
///     "[1, 2, ... (2 remaining)]"
/// );
/// ```
fn abbreviate_u8_slice_as_bytes(slice: &[u8], maximum_displayed_length: usize) -> String {
    if slice.len() <= maximum_displayed_length {
        return format!("{:?}", slice);
    }


    let mut elements = Vec::with_capacity(slice.len().min(maximum_displayed_length));

    for element in slice.iter().take(maximum_displayed_length) {
        elements.push(format!("{}", *element));
    }


    format!(
        "[{}, ... ({} remaining)]",
        elements.join(", "),
        slice.len().saturating_sub(maximum_displayed_length)
    )
}


/// Formats the given byte `slice` as an UTF-8 string, up to the specified `maximum_displayed_chars`.
///
/// If the parsed string is longer than the maximum allowed characters, the rest are not displayed.
///
/// If the slice is not valid UTF-8, the "(not UTF-8)" string is returned.
///
/// ```ignore
/// assert_eq!(
///     abbreviate_u8_slice_as_string(&[72, 69, 76, 76, 79, 32, 87, 79, 82, 76, 68], 7),
///     "HELLO W [4 remaining]"
/// );
/// ```
fn abbreviate_u8_slice_as_string(slice: &[u8], maximum_displayed_chars: usize) -> String {
    let Ok(as_string) = String::from_utf8(slice.to_vec()) else {
        return "(not UTF-8)".to_string();
    };


    format!(
        "{} [{} remaining]",
        as_string
            .chars()
            .take(maximum_displayed_chars)
            .collect::<String>(),
        as_string
            .chars()
            .count()
            .saturating_sub(maximum_displayed_chars)
    )
}


/// A file state (non-existent / empty / with some data).
///
/// See also: [`CapturedFileState`].
#[derive(Clone, PartialEq, Eq)]
pub enum FileState {
    NonExistent,
    Empty,
    NonEmpty { content: Vec<u8> },
}

impl FileState {
    /// Captures the file state for the given `file_path`.
    ///
    /// # Panics
    /// Panics if the file cannot be accessed.
    ///
    /// Panicking is accepted for this function only because
    /// this is part of `fs-more`'s testing harness,
    /// and we *should panic on errors anyway*.
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


    /// Returns `true` if the file states equal.
    #[inline]
    pub fn equals_other_file_state(&self, other: &Self) -> bool {
        self == other
    }
}


impl Debug for FileState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileState::NonExistent => write!(f, "FileState::NonExistent"),
            FileState::Empty => write!(f, "FileState::Empty"),
            FileState::NonEmpty { content } => {
                write!(
                    f,
                    "FileState::NonEmpty {{\n  \
                        {}: {}\n  \
                        {}\n\
                    }}",
                    humansize::format_size(content.len(), humansize::BINARY),
                    abbreviate_u8_slice_as_bytes(content, 12),
                    abbreviate_u8_slice_as_string(content, 12)
                )
            }
        }
    }
}



/// A contextual file state (non-existent / empty / with some data).
///
/// Unlike [`FileState`], this struct also carries the information
/// about the path of the captured file, allowing us to
/// e.g. [`CapturedFileState::assert_unchanged`].
pub struct CapturedFileState {
    file_path: PathBuf,

    captured_state: FileState,
}


impl CapturedFileState {
    /// Captures the file state at the given file `path`.
    ///
    /// # Panics
    /// Panics if the file cannot be accessed.
    ///
    /// Panicking is accepted for this function only because
    /// this is part of `fs-more`'s testing harness,
    /// and we *should panic on errors anyway*.
    pub fn new_with_content_capture<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            file_path: path.as_ref().to_path_buf(),
            captured_state: FileState::capture_from_file_path(path),
        }
    }

    /// Initialises a new captured file state by manually providing a [`FileState`].
    pub fn new_with_manual_state<P>(path: P, state: FileState) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            file_path: path.as_ref().to_path_buf(),
            captured_state: state,
        }
    }

    /// Returns the path of the captured file.
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Asserts the captured state is equal to the state of the `other` state.
    /// Does not compare paths, only states.
    pub fn assert_captured_states_equal(&self, other: &Self) {
        assert!(
            self.captured_state
                .equals_other_file_state(&other.captured_state),
            "files \"{}\" and \"{}\" don't have equal states",
            self.file_path.display(),
            other.file_path.display(),
        )
    }

    /// Asserts the `other` file's current state on disk
    /// matches the captured `self` state.
    pub fn assert_captured_state_matches_other_file<P>(&self, other: P)
    where
        P: AsRef<Path>,
    {
        let captured_other_file = Self::new_with_content_capture(other);
        self.assert_captured_states_equal(&captured_other_file);
    }

    /// Asserts the file's current state on disk
    /// matches the captured state.
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


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn properly_abbreviates_u8_slice() {
        assert_eq!(
            abbreviate_u8_slice_as_bytes(&[1, 2, 3, 4], 2),
            "[1, 2, ... (2 remaining)]"
        );
    }

    #[test]
    fn properly_abbreviates_u8_slice_as_string() {
        assert_eq!(
            abbreviate_u8_slice_as_string(&[72, 69, 76, 76, 79, 32, 87, 79, 82, 76, 68], 7),
            "HELLO W [4 remaining]"
        );
    }
}
