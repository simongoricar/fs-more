//! File sizing, copying, moving and removal operations. Includes progress monitoring variants.

use std::path::{Path, PathBuf};

use_enabled_fs_module!();

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

use crate::{error::FileError, use_enabled_fs_module};


/// Rules that dictate how existing files are handled when copying or moving.
///
/// See also: [`CopyFileOptions`] and [`MoveFileOptions`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ExistingFileBehaviour {
    /// Ensures that an error will be returned from the corresponding
    /// file copying or moving function when a destination file already exists.
    Abort,

    /// Ensures that an existing destination file will not be overwritten
    /// by the corresponding copying or moving function.
    ///
    /// However, the function will skip the file silently; no error will be returned.
    Skip,

    /// Ensures that an existing destination file *can* be overwritten
    /// by the corresponding copying or moving function.
    Overwrite,
}



pub(crate) struct ValidatedSourceFilePath {
    /// Canonical source file path.
    pub(crate) source_file_path: PathBuf,

    /// Indicates that the file at `source_file_path` **must not** be moved.
    ///
    /// **This flag is relevant only if the operation happens to be moving a file.**
    ///
    /// This flag can be `true` in cases when the `source_file_path` was a symlink to a file and we
    /// canonicalized the path in `validate_source_file_path`, meaning the path in this struct no longer points to the
    /// user-provided symlink, but to the file that link points to. In that case, we must not move the file, but copy it,
    /// and then delete the original symbolic link the user wanted to move.
    pub(crate) original_was_symlink_to_file: bool,
}


/// Given a source file path, validate that it exists on the file system and is truly a file.
///
/// If the given path is a symlink to a file, the returned path will be a resolved (canonical) one,
/// i.e. pointing to the real file.
fn validate_source_file_path(
    source_file_path: &Path,
) -> Result<ValidatedSourceFilePath, FileError> {
    // Ensure the source file path exists. We use `try_exists`
    // instead of `exists` to catch permission and other IO errors
    // as distinct from the `FileError::NotFound` error.

    match source_file_path.try_exists() {
        Ok(exists) => {
            if !exists {
                return Err(FileError::SourceFileNotFound {
                    path: source_file_path.to_path_buf(),
                });
            }

            if !source_file_path.is_file() {
                return Err(FileError::SourcePathNotAFile {
                    path: source_file_path.to_path_buf(),
                });
            }

            if source_file_path.is_symlink() {
                let canonicalized_path = fs::canonicalize(source_file_path).map_err(|error| {
                    FileError::UnableToCanonicalizeSourceFilePath {
                        path: source_file_path.to_path_buf(),
                        error,
                    }
                })?;

                #[cfg(feature = "dunce")]
                {
                    let de_unced_canonicalized_path =
                        dunce::simplified(&canonicalized_path).to_path_buf();

                    return Ok(ValidatedSourceFilePath {
                        source_file_path: de_unced_canonicalized_path,
                        original_was_symlink_to_file: true,
                    });
                }

                #[cfg(not(feature = "dunce"))]
                {
                    return Ok(ValidatedSourceFilePath {
                        source_file_path: canonicalized_path,
                        original_was_symlink_to_file: true,
                    });
                }
            }

            Ok(ValidatedSourceFilePath {
                source_file_path: source_file_path.to_path_buf(),
                original_was_symlink_to_file: false,
            })
        }
        Err(error) => Err(FileError::UnableToAccessSourceFile {
            path: source_file_path.to_path_buf(),
            error,
        }),
    }
}


pub(crate) struct ValidatedDestinationFilePath {
    /// Canonical destination file path.
    pub(crate) destination_file_path: PathBuf,

    pub(crate) exists: bool,
}

pub(crate) enum DestinationValidationAction {
    /// The validation logic concluded that no action should be taken
    /// (the file should not be copied or moved) since the destination file already exists,
    /// and `existing_destination_file_behaviour` is set to [`ExistingFileBehaviour::Skip`].
    SkipCopyOrMove,

    /// The validation logic found no errors.
    Continue(ValidatedDestinationFilePath),
}

fn validate_destination_file_path(
    validated_source_file_path: &ValidatedSourceFilePath,
    destination_file_path: &Path,
    existing_destination_file_behaviour: ExistingFileBehaviour,
) -> Result<DestinationValidationAction, FileError> {
    // Ensure the destination file path doesn't exist yet
    // (unless `options.existing_destination_file_behaviour` allows that),
    // and that it isn't a directory.

    let destination_file_exists = destination_file_path.try_exists().map_err(|error| {
        FileError::UnableToAccessDestinationFile {
            path: destination_file_path.to_path_buf(),
            error,
        }
    })?;


    if destination_file_exists {
        let canonical_destination_path = {
            let canonical_destination_path =
                destination_file_path.canonicalize().map_err(|error| {
                    FileError::UnableToCanonicalizeDestinationFilePath {
                        path: destination_file_path.to_path_buf(),
                        error,
                    }
                })?;

            #[cfg(feature = "dunce")]
            {
                dunce::simplified(&canonical_destination_path).to_path_buf()
            }

            #[cfg(not(feature = "dunce"))]
            {
                canonical_destination_path
            }
        };


        // Ensure we don't try to copy the file into itself.

        if validated_source_file_path
            .source_file_path
            .eq(&canonical_destination_path)
        {
            return Err(FileError::SourceAndDestinationAreTheSame {
                path: canonical_destination_path,
            });
        }


        // Ensure we respect the [`ExistingFileBehaviour`] option if
        // the destination file already exists.
        if destination_file_exists {
            match existing_destination_file_behaviour {
                ExistingFileBehaviour::Abort => {
                    return Err(FileError::DestinationPathAlreadyExists {
                        path: destination_file_path.to_path_buf(),
                    })
                }
                ExistingFileBehaviour::Skip => {
                    return Ok(DestinationValidationAction::SkipCopyOrMove);
                }
                ExistingFileBehaviour::Overwrite => {}
            };
        }


        Ok(DestinationValidationAction::Continue(
            ValidatedDestinationFilePath {
                destination_file_path: canonical_destination_path,
                exists: true,
            },
        ))
    } else {
        Ok(DestinationValidationAction::Continue(
            ValidatedDestinationFilePath {
                destination_file_path: destination_file_path.to_path_buf(),
                exists: false,
            },
        ))
    }
}
