use std::path::Path;

use super::validate_source_file_path;
use crate::error::FileError;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileMoveOptions {
    pub overwrite_existing: bool,
}


pub fn move_file<P, T>(
    source_file_path: P,
    target_file_path: T,
    options: &FileMoveOptions,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let target_file_path = target_file_path.as_ref();

    validate_source_file_path(source_file_path)?;

    // Ensure the target file path doesn't exist yet
    // (unless `overwrite_existing` is `true`)
    // and that it isn't already a directory path.
    match target_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path =
                    source_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToCanonicalizeSourcePath { error }
                    })?;
                let canonicalized_target_path =
                    target_file_path.canonicalize().map_err(|error| {
                        FileError::UnableToCanonicalizeTargetPath { error }
                    })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndTargetAreTheSameFile);
                }
            }

            if exists && !options.overwrite_existing {
                return Err(FileError::AlreadyExists);
            }
        }
        Err(error) => return Err(FileError::UnableToAccessTargetFile { error }),
    }

    // All checks have passed. Now we do the following:
    // - if both paths reside on the same filesystem
    //   (as indicated by std::fs::rename succeeding) that's nice (and fast),
    // - otherwise we need to copy to target and remove source.

    if std::fs::rename(source_file_path, target_file_path).is_ok() {
        // Get size of file that we just renamed.
        let target_file_path_metadata = target_file_path
            .metadata()
            .map_err(|error| FileError::OtherIoError { error })?;

        Ok(target_file_path_metadata.len())
    } else {
        // Copy, then delete original.
        let num_bytes_copied = std::fs::copy(source_file_path, target_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        std::fs::remove_file(source_file_path)
            .map_err(|error| FileError::OtherIoError { error })?;

        Ok(num_bytes_copied)
    }
}
