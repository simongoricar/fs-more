use std::path::Path;

use crate::error::FileSizeError;

/// Retrieve the size of a file in bytes.
///
/// ## Symbolic links
/// If the provided `file_path` is a symbolic link leading to a file,
/// the size of the file the link is pointing to, not of the link itself, is returned.
pub fn file_size_in_bytes<P>(file_path: P) -> Result<u64, FileSizeError>
where
    P: AsRef<Path>,
{
    let file_path = file_path.as_ref();

    // Ensure the file exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileMetadataError::NotFound` error.
    match file_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(FileSizeError::NotFound);
            }
        }
        Err(error) => {
            return Err(FileSizeError::UnableToAccessFile { error });
        }
    }

    if !file_path.is_file() && !file_path.is_symlink() {
        return Err(FileSizeError::NotAFile);
    }

    // This follows symbolic links, but we must recheck that
    // what it leads to is also a file.
    let file_metadata = file_path
        .metadata()
        .map_err(|error| FileSizeError::OtherIoError { error })?;

    if !file_metadata.is_file() {
        return Err(FileSizeError::NotAFile);
    }

    Ok(file_metadata.len())
}
