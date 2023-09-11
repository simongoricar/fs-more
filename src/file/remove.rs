use std::path::Path;

use crate::error::FileRemoveError;

/// Removes a single file.
///
/// ## Internals
/// This function uses [`std::fs::remove_file`] internally.
pub fn remove_file<P>(file_path: P) -> Result<(), FileRemoveError>
where
    P: AsRef<Path>,
{
    let file_path = file_path.as_ref();

    // Ensure the source file path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileError::NotFound` error.
    match file_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(FileRemoveError::NotFound);
            }
        }
        Err(error) => {
            return Err(FileRemoveError::UnableToAccessFile { error });
        }
    }

    if !file_path.is_file() {
        return Err(FileRemoveError::NotAFile);
    }

    // All checks have passed, remove the file.
    std::fs::remove_file(file_path)
        .map_err(|error| FileRemoveError::OtherIoError { error })?;

    Ok(())
}
