use std::{
    fs::{self, FileType, OpenOptions},
    io::{prelude::Read, BufReader},
    path::{Path, PathBuf},
};

use thiserror::Error;



/// An internal error that can ocurr when comparing files.
#[derive(Debug, Error)]
pub(crate) enum FileComparisonErrorInner {
    #[error(
        "unable to read metadata for path \"{}\"",
        .path.display()
    )]
    UnableToReadMetadata {
        path: PathBuf,

        #[source]
        error: std::io::Error,
    },

    #[error(
        "file types don't match or are not files (strict symlink comparison = {}): \
        first path is {:?}, second is {:?}",
        .strict_symlink_comparison_enabled,
        .first_path_type,
        .second_path_type
    )]
    TypeMismatch {
        strict_symlink_comparison_enabled: bool,

        first_path_type: FileType,

        second_path_type: FileType,
    },

    #[error(
        "unable to open or read file: \"{}\"",
        .file_path.display()
    )]
    UnableToReadFile {
        file_path: PathBuf,

        #[source]
        error: std::io::Error,
    },

    #[error(
        "files do not match at byte {}: first has {}, second has {}",
        .byte_index,
        .first_file_value,
        .second_file_value
    )]
    ByteDoesNotMatch {
        byte_index: usize,

        first_file_value: u8,

        second_file_value: u8,
    },
}



/// File comparison options.
#[derive(Clone, Debug)]
pub struct FileComparisonOptions {
    /// If `true`, the comparison will require that
    /// symlinks to files on one side must be
    /// symlinks on the other as well. The contents of symlink
    /// destinations will then be compared.
    ///
    /// If `false`, the comparison will not mind if a symlink to a file
    /// on one side is actually a direct file or directory
    /// on the other, as long as the contents match.
    pub strict_symlink_comparison: bool,
}


/// Given two paths, this method ensures they are file paths
/// and that their contents are identical.
///
/// This handles symlinks leading to files (see `options`), as well as
/// does proper error handling when either of the paths is invalid
/// (e.g. points to a directory).
///
/// See [`FileComparisonErrorInner`] for possible errors.
pub(crate) fn ensure_contents_of_files_are_equal_inner(
    first_file_path: &Path,
    second_file_path: &Path,
    options: FileComparisonOptions,
) -> Result<(), FileComparisonErrorInner> {
    let first_file_metadata_no_follow = fs::symlink_metadata(first_file_path).map_err(|error| {
        FileComparisonErrorInner::UnableToReadMetadata {
            path: first_file_path.to_path_buf(),
            error,
        }
    })?;

    let second_file_metadata_no_follow =
        fs::symlink_metadata(second_file_path).map_err(|error| {
            FileComparisonErrorInner::UnableToReadMetadata {
                path: second_file_path.to_path_buf(),
                error,
            }
        })?;


    #[allow(clippy::if_same_then_else)]
    if options.strict_symlink_comparison {
        if first_file_metadata_no_follow.is_symlink()
            && !second_file_metadata_no_follow.is_symlink()
        {
            return Err(FileComparisonErrorInner::TypeMismatch {
                strict_symlink_comparison_enabled: options.strict_symlink_comparison,
                first_path_type: first_file_metadata_no_follow.file_type(),
                second_path_type: second_file_metadata_no_follow.file_type(),
            });
        } else if second_file_metadata_no_follow.is_symlink()
            && !first_file_metadata_no_follow.is_symlink()
        {
            return Err(FileComparisonErrorInner::TypeMismatch {
                strict_symlink_comparison_enabled: options.strict_symlink_comparison,
                first_path_type: first_file_metadata_no_follow.file_type(),
                second_path_type: second_file_metadata_no_follow.file_type(),
            });
        }
    } else if !first_file_path.is_file() || !second_file_path.is_file() {
        return Err(FileComparisonErrorInner::TypeMismatch {
            strict_symlink_comparison_enabled: options.strict_symlink_comparison,
            first_path_type: first_file_metadata_no_follow.file_type(),
            second_path_type: second_file_metadata_no_follow.file_type(),
        });
    }



    const FILE_READ_BUFFER_SIZE: usize = 1024 * 16;

    let first_file = {
        let file = OpenOptions::new()
            .read(true)
            .open(first_file_path)
            .map_err(|error| FileComparisonErrorInner::UnableToReadFile {
                file_path: first_file_path.to_path_buf(),
                error,
            })?;

        BufReader::with_capacity(FILE_READ_BUFFER_SIZE, file)
    };

    let second_file = {
        let file = OpenOptions::new()
            .read(true)
            .open(second_file_path)
            .map_err(|error| FileComparisonErrorInner::UnableToReadFile {
                file_path: second_file_path.to_path_buf(),
                error,
            })?;

        BufReader::with_capacity(FILE_READ_BUFFER_SIZE, file)
    };


    // Compare file contents.
    for (byte_index, (first_file_value, second_file_value)) in
        first_file.bytes().zip(second_file.bytes()).enumerate()
    {
        let first_file_value =
            first_file_value.map_err(|error| FileComparisonErrorInner::UnableToReadFile {
                file_path: first_file_path.to_path_buf(),
                error,
            })?;

        let second_file_value =
            second_file_value.map_err(|error| FileComparisonErrorInner::UnableToReadFile {
                file_path: second_file_path.to_path_buf(),
                error,
            })?;


        if first_file_value != second_file_value {
            return Err(FileComparisonErrorInner::ByteDoesNotMatch {
                byte_index,
                first_file_value,
                second_file_value,
            });
        }
    }

    Ok(())
}
