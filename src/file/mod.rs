//! File copying, moving and removal operations. Includes progress monitoring variants.

use std::path::Path;

mod copy;
mod r#move;
mod progress;
mod remove;

pub use copy::{
    copy_file,
    copy_file_with_progress,
    FileCopyOptions,
    FileCopyWithProgressOptions,
};
pub use progress::{FileProgress, ProgressWriter};
pub use r#move::{move_file, FileMoveOptions};
pub use remove::remove_file;

use crate::error::FileError;

/// Given a `&Path`, validate that it exists and is a file.
fn validate_source_file_path(source_file_path: &Path) -> Result<(), FileError> {
    // Ensure the source file path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileError::NotFound` error.
    match source_file_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(FileError::NotFound);
            }
        }
        Err(error) => {
            return Err(FileError::UnableToAccessSourceFile { error });
        }
    }

    if !source_file_path.is_file() {
        return Err(FileError::NotAFile);
    }

    Ok(())
}
