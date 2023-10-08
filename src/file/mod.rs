//! File sizing, copying, moving and removal operations. Includes progress monitoring variants.

#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "fs-err")]
use fs_err as fs;

mod copy;
mod r#move;
mod progress;
mod remove;
mod size;

pub use copy::*;
pub use progress::*;
pub use r#move::*;
pub use remove::*;
pub use size::*;

use crate::error::FileError;

pub(crate) struct ValidatedSourceFilePath {
    pub(crate) source_file_path: PathBuf,

    /// Indicates that the file at `source_file_path` **must not** be moved.
    /// **This flag is relevant only if the operation happens to be moving a file.**
    ///
    /// This flag can be `true` in cases when the user-provided `source_file_path` was a symlink to a file and we
    /// canonicalized the path in `validate_source_file_path`, meaning the path in this struct no longer points to the
    /// user-provided symlink, but to the file that link points to. In that case, we must not move the file, but copy it,
    /// and then delete the original symbolic link the user wanted to move.
    pub(crate) original_was_symlink_to_file: bool,
}

/// Given a `&Path`, validate that it exists and is a file.
/// If the given path is a symlink to a file, the returned path will be a resolved one, i.e. pointing to the real file.
fn validate_source_file_path(
    source_file_path: &Path,
) -> Result<ValidatedSourceFilePath, FileError> {
    // Ensure the source file path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileError::NotFound` error.
    match source_file_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(FileError::NotFound);
            }

            if !source_file_path.is_file() {
                return Err(FileError::NotAFile);
            }

            if source_file_path.is_symlink() {
                let canonicalized_path = fs::canonicalize(source_file_path)
                    .map_err(|error| FileError::UnableToAccessSourceFile { error })?;

                return Ok(ValidatedSourceFilePath {
                    source_file_path: canonicalized_path,
                    original_was_symlink_to_file: true,
                });
            }

            Ok(ValidatedSourceFilePath {
                source_file_path: source_file_path.to_path_buf(),
                original_was_symlink_to_file: false,
            })
        }
        Err(error) => Err(FileError::UnableToAccessSourceFile { error }),
    }
}
