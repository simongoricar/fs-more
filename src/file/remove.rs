use std::path::Path;

use_enabled_fs_module!();

use crate::error::FileRemoveError;

/// Removes a single file.
///
///
/// # Symbolic link behaviour
/// Symbolic links are not followed.
///
/// This means that, if `source_file_path` is a valid symbolic link to a file,
/// the *link* at the source file path will be removed, not the target file the link points to.
/// If the symlink is broken, or points to something other than a file, an error is returned.
///
///
/// # Errors
/// If the file cannot be removed, a [`FileRemoveError`] is returned;
/// see its documentation for more details.
/// Here is a non-exhaustive list of error causes:
/// - If the file does not exist, a [`NotFound`] variant is returned.
/// - If the path exists, but is not a file, [`NotAFile`] is returned.
///   Notably, this is also returned when the path is a symlink to something other than a file.
/// - If there is an issue accessing the file, for example due to missing permissions,
///   then a [`UnableToAccessFile`] is returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// <br>
///
/// <details>
/// <summary><h4>Implementation details</h4></summary>
///
/// *This section describes internal implementations details.
/// They should not be relied on, because they are informative
/// and may change in the future.*
///
/// <br>
///
/// This function currently uses [`std::fs::remove_file`] internally
/// (or [`fs_err::remove_file`](https://docs.rs/fs-err/latest/fs_err/fn.remove_file.html)
/// if the `fs-err` feature flag is enabled).
///
/// </details>
///
///
/// [`NotFound`]: FileRemoveError::NotFound
/// [`NotAFile`]: FileRemoveError::NotAFile
/// [`UnableToAccessFile`]: FileRemoveError::UnableToAccessFile
/// [`OtherIoError`]: FileRemoveError::OtherIoError
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
                return Err(FileRemoveError::NotFound {
                    path: file_path.to_path_buf(),
                });
            }
        }
        Err(error) => {
            return Err(FileRemoveError::UnableToAccessFile {
                path: file_path.to_path_buf(),
                error,
            });
        }
    }


    let file_metadata =
        fs::symlink_metadata(file_path).map_err(|error| FileRemoveError::UnableToAccessFile {
            path: file_path.to_path_buf(),
            error,
        })?;

    if file_metadata.is_symlink() {
        let file_metadata_resolved =
            fs::metadata(file_path).map_err(|error| FileRemoveError::UnableToAccessFile {
                path: file_path.to_path_buf(),
                error,
            })?;

        if !file_metadata_resolved.is_file() {
            return Err(FileRemoveError::NotAFile {
                path: file_path.to_path_buf(),
            });
        }
    } else if !file_metadata.is_file() {
        return Err(FileRemoveError::NotAFile {
            path: file_path.to_path_buf(),
        });
    }

    // All checks have passed, remove the file.
    fs::remove_file(file_path).map_err(|error| FileRemoveError::OtherIoError { error })?;

    Ok(())
}
