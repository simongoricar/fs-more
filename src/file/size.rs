use std::path::Path;

use_enabled_fs_module!();

use crate::{error::FileSizeError, use_enabled_fs_module};

/// Retrieve the size of a file in bytes.
///
/// ## Symbolic link behaviour
/// Symbolic links are resolved, meaning that, if the provided `file_path` is
/// a symbolic link leading to a file, the function returns
/// *the size of the file, not of the link itself*.
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
                return Err(FileSizeError::NotFound {
                    path: file_path.to_path_buf(),
                });
            }
        }
        Err(error) => {
            return Err(FileSizeError::UnableToAccessFile {
                file_path: file_path.to_path_buf(),
                error,
            });
        }
    }

    // DEPRECATED I don't think we need this? We cover the symlink case just below.
    if !file_path.is_file() && !file_path.is_symlink() {
        return Err(FileSizeError::NotAFile {
            path: file_path.to_path_buf(),
        });
    }

    // This follows symbolic links, but we must recheck that
    // what it leads to is also a file.
    let file_metadata =
        fs::metadata(file_path).map_err(|error| FileSizeError::OtherIoError { error })?;

    if !file_metadata.is_file() {
        return Err(FileSizeError::NotAFile {
            path: file_path.to_path_buf(),
        });
    }

    Ok(file_metadata.len())
}
