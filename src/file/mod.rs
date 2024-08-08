//! File copying, moving, sizing and removal operations.
//! *Includes progress monitoring variants.*
//!
//! <br>
//!
//! ##### Feature Overview
//!
//! | | <span style="font-weight:normal"><i>configured by</i></span> | <span style="font-weight:normal"><i>returns</i></span> |
//! |-----------------------------|---------------------------------|:--------------------:|
//! | [`copy_file`]               | [`FileCopyOptions`]             | [`FileCopyFinished`] <br><sup style="text-align: right">(or [`FileError`])</sup> |
//! | [`copy_file_with_progress`] | [`FileCopyWithProgressOptions`] | [`FileCopyFinished`] <br><sup style="text-align: right">(or [`FileError`])</sup> |
//! | [`move_file`]               | [`FileMoveOptions`]             | [`FileMoveFinished`] <br><sup style="text-align: right">(or [`FileError`])</sup> |
//! | [`move_file_with_progress`] | [`FileMoveWithProgressOptions`] | [`FileMoveFinished`] <br><sup style="text-align: right">(or [`FileError`])</sup> |
//! | [`remove_file`]             |                                 | `()` <br><sup style="text-align: right">(or [`FileRemoveError`])</sup> |
//! | [`file_size_in_bytes`]      |                                 | [`u64`] <br><sup style="text-align: right">(or [`FileSizeError`])</sup> |
//!
//!
//! [`FileError`]: crate::error::FileError
//! [`FileRemoveError`]: crate::error::FileRemoveError
//! [`FileSizeError`]: crate::error::FileSizeError

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

use crate::{directory::try_exists_without_follow, error::FileError};


/// Controls behaviour for existing files on the destination side
/// that collide with the one we're trying to copy or move there.
///
/// See also: [`FileCopyOptions`] and [`FileMoveOptions`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollidingFileBehaviour {
    /// Ensures that an error will be returned from the corresponding function
    /// when the destination file already exists.
    Abort,

    /// Ensures that an existing destination file will not be overwritten
    /// by the corresponding copy or move operation.
    ///
    /// However, the function will skip the file silently; no error will be returned.
    Skip,

    /// Ensures that an existing destination file *can* be overwritten
    /// by the corresponding copying or moving function.
    Overwrite,
}



/// A set of paths and auxiliary information about a source file path.
pub(crate) struct ValidatedSourceFilePath {
    /// Canonical source file path.
    ///
    /// If the original file path was a symlink leading to some target file,
    /// this path points to that target file.
    pub(crate) source_file_path: PathBuf,

    /// Indicates whether the original source file path (before canonicalization)
    /// was a symlink to a file.
    ///
    /// **This flag is relevant only if the operation happens to be moving a file.**
    ///
    /// This flag is be `true` when the original `source_file_path` was a symlink to a file and we
    /// canonicalized the path in [`validate_source_file_path`].
    ///
    /// This means the path in this struct no longer points to the symlink,
    /// but to the file that link itself points to. In that case, we must not move the file,
    /// but copy it and then delete the original symlink the user wanted to move.
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

    let source_file_exists = match try_exists_without_follow(source_file_path) {
        Ok(exists) => exists,
        Err(error) => {
            return Err(FileError::UnableToAccessSourceFile {
                path: source_file_path.to_path_buf(),
                error,
            });
        }
    };


    if !source_file_exists {
        return Err(FileError::SourceFileNotFound {
            path: source_file_path.to_path_buf(),
        });
    }

    if !source_file_path.is_file() {
        return Err(FileError::SourcePathNotAFile {
            path: source_file_path.to_path_buf(),
        });
    }


    let canonical_path = fs::canonicalize(source_file_path).map_err(|error| {
        FileError::UnableToAccessSourceFile {
            path: source_file_path.to_path_buf(),
            error,
        }
    })?;

    #[cfg(feature = "dunce")]
    {
        let de_unced_canonical_path = dunce::simplified(&canonical_path).to_path_buf();

        Ok(ValidatedSourceFilePath {
            source_file_path: de_unced_canonical_path,
            original_was_symlink_to_file: true,
        })
    }

    #[cfg(not(feature = "dunce"))]
    {
        Ok(ValidatedSourceFilePath {
            source_file_path: canonical_path,
            original_was_symlink_to_file: true,
        })
    }
}


/// A set of paths and auxiliary information about a destination file path.
pub(crate) struct ValidatedDestinationFilePath {
    /// Canonical destination file path.
    ///
    /// If the original file path was a symlink leading to some target file,
    /// this path points to that target file.
    pub(crate) destination_file_path: PathBuf,

    /// Whether the destination already exists.
    pub(crate) exists: bool,
}

pub(crate) enum DestinationValidationAction {
    /// The validation logic concluded that no action should be taken
    /// (the file should not be copied or moved) since the destination file already exists,
    /// and `colliding_file_behaviour` is set to [`CollidingFileBehaviour::Skip`].
    SkipCopyOrMove,

    /// The validation logic found no path validation errors.
    Continue(ValidatedDestinationFilePath),
}


/// Given a destination file path, validate that it respects `colliding_file_behaviour`,
/// and that if it is a symlink, that it points to a file.
///
/// If the given path is a symlink to a file, the returned path will be a resolved (canonical) one,
/// i.e. pointing to the real file.
fn validate_destination_file_path(
    validated_source_file_path: &ValidatedSourceFilePath,
    destination_file_path: &Path,
    colliding_file_behaviour: CollidingFileBehaviour,
) -> Result<DestinationValidationAction, FileError> {
    // Ensure the destination file path doesn't exist yet
    // (unless `options.colliding_file_behaviour` allows that),
    // and that it isn't a directory.

    let destination_file_exists =
        try_exists_without_follow(destination_file_path).map_err(|error| {
            FileError::UnableToAccessDestinationFile {
                path: destination_file_path.to_path_buf(),
                error,
            }
        })?;


    if destination_file_exists {
        let canonical_destination_path = {
            let canonical_destination_path =
                destination_file_path.canonicalize().map_err(|error| {
                    FileError::UnableToAccessDestinationFile {
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


        // Ensure we respect the [`CollidingFileBehaviour`] option if
        // the destination file already exists.
        if destination_file_exists {
            match colliding_file_behaviour {
                CollidingFileBehaviour::Abort => {
                    return Err(FileError::DestinationPathAlreadyExists {
                        path: destination_file_path.to_path_buf(),
                    })
                }
                CollidingFileBehaviour::Skip => {
                    return Ok(DestinationValidationAction::SkipCopyOrMove);
                }
                CollidingFileBehaviour::Overwrite => {}
            };
        }


        Ok(DestinationValidationAction::Continue(ValidatedDestinationFilePath {
            destination_file_path: canonical_destination_path,
            exists: true,
        }))
    } else {
        Ok(DestinationValidationAction::Continue(ValidatedDestinationFilePath {
            destination_file_path: destination_file_path.to_path_buf(),
            exists: false,
        }))
    }
}
