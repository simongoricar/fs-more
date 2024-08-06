use std::path::Path;

use_enabled_fs_module!();

use crate::{directory::try_exists_without_follow, error::FileSizeError};

/// Retrieve the size of a file in bytes.
///
///
/// ## Symbolic link behaviour
/// Symbolic links are not followed.
///
/// This matches the behaviour of `du` on Unix[^unix-du].
///
///
/// # Errors
/// If the size of the file cannot be retrieved, a [`FileSizeError`] is returned;
/// see its documentation for more details.
/// Here is a non-exhaustive list of error causes:
/// - If the file does not exist, a [`NotFound`] variant is returned.
/// - If the path exists, but is not a file, [`NotAFile`] is returned.
/// - If there is an issue accessing the file, for example due to missing permissions,
///   then a [`UnableToAccessFile`] is returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// [`NotFound`]: FileSizeError::NotFound
/// [`NotAFile`]: FileSizeError::NotAFile
/// [`UnableToAccessFile`]: FileSizeError::UnableToAccessFile
/// [`OtherIoError`]: FileSizeError::OtherIoError
/// [^unix-du]: Source for coreutils' `du` is available
///     [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/du.c).
pub fn file_size_in_bytes<P>(file_path: P) -> Result<u64, FileSizeError>
where
    P: AsRef<Path>,
{
    let file_path = file_path.as_ref();

    // TODO Make symbolic link behaviour configurable here (and write tests for that).

    // Ensure the file exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileMetadataError::NotFound` error.

    match try_exists_without_follow(file_path) {
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
