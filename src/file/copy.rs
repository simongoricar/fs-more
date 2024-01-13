#[cfg(not(feature = "fs-err"))]
use std::fs;
use std::{
    io::{BufReader, BufWriter, Write},
    path::Path,
};

#[cfg(feature = "fs-err")]
use fs_err as fs;

use super::{
    progress::{FileProgress, ProgressWriter},
    validate_source_file_path,
    ValidatedSourceFilePath,
};
use crate::error::FileError;


/// Options that influence the [`copy_file`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileCopyOptions {
    /// Whether to overwrite an existing target file if it exists already.
    ///
    /// Note that this has lower precedence than `skip_existing`.
    pub overwrite_existing: bool,

    /// Whether to skip copying the file if it already exists.
    ///
    /// This takes precedence over `overwrite_existing`.
    pub skip_existing: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FileCopyOptions {
    fn default() -> Self {
        Self {
            overwrite_existing: false,
            skip_existing: false,
        }
    }
}


/// Copy a single file from the `source_file_path` to the `target_file_path`.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
///
/// ## Special return value cases
/// If copying is skipped due to and existing file and
/// [`options.skip_existing`][FileCopyOptions::skip_existing],
/// the return value will be `Ok(0)`.
///
///
/// ## Symbolic links
/// If `source_file_path` is a symbolic link to a file, the contents of the file it points to will be copied to `target_file_path`
/// (same behaviour as `cp` without `-P` on Unix).
///
///
/// ## Internals
/// This function internally delegates copying to [`std::fs::copy`] from the standard library
/// (but note that [`copy_file_with_progress`] does not).
pub fn copy_file<P, T>(
    source_file_path: P,
    target_file_path: T,
    options: FileCopyOptions,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let target_file_path = target_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path, ..
    } = validate_source_file_path(source_file_path)?;

    // Ensure the target file path doesn't exist yet
    // (unless `overwrite_existing` is `true`)
    // and that it isn't already a directory path.
    match target_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path = source_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessSourceFile { error })?;
                let canonicalized_target_path = target_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessTargetFile { error })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndTargetAreTheSameFile);
                }
            }

            if exists && options.skip_existing {
                return Ok(0);
            }

            if exists && !options.overwrite_existing {
                return Err(FileError::AlreadyExists);
            }
        }
        Err(error) => return Err(FileError::UnableToAccessTargetFile { error }),
    }

    // All checks have passed, pass the copying onto Rust's standard library.
    let num_bytes_copied = fs::copy(source_file_path, target_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;

    Ok(num_bytes_copied)
}


/// Options that influence the [`copy_file_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileCopyWithProgressOptions {
    /// Whether to overwrite an existing target file if it exists already.
    ///
    /// Note that this has lower precedence than `skip_existing`.
    pub overwrite_existing: bool,

    /// Whether to skip copying the file if it already exists.
    /// This takes precedence over `overwrite_existing`.
    pub skip_existing: bool,

    /// Internal buffer size (for both reading and writing) when copying the file,
    /// defaults to 64 KiB.
    pub buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
    ///
    /// *Note that the interval can be larger.*
    pub progress_update_byte_interval: u64,
}

impl Default for FileCopyWithProgressOptions {
    fn default() -> Self {
        Self {
            overwrite_existing: false,
            skip_existing: false,
            // 64 KiB
            buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Copies the specified file from the source to the target with the specified options.
///
/// This is done by opening both files (one for reading, another for writing) and wrapping
/// them in buffered readers and writers and our progress tracker and then using
/// the [`std::io::copy`] function to copy the entire file.
///
/// *Warning:* no checks are performed before copying
/// (e.g. whether source exists or whether target is a directory or already exists).
pub(crate) fn copy_file_with_progress_unchecked<F>(
    source_file_path: &Path,
    target_file_path: &Path,
    options: FileCopyWithProgressOptions,
    progress_handler: F,
) -> Result<u64, FileError>
where
    F: FnMut(&FileProgress),
{
    let bytes_total = fs::metadata(source_file_path)
        .map_err(|error| FileError::OtherIoError { error })?
        .len();

    // Open a file for reading and a file for writing,
    // wrap them in buffers and progress monitors, then copy the file.
    let input_file = fs::OpenOptions::new()
        .read(true)
        .open(source_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;

    let mut input_file_buffered = BufReader::with_capacity(options.buffer_size, input_file);


    let output_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(target_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;

    let output_file_progress_monitored = ProgressWriter::new(
        output_file,
        progress_handler,
        options.progress_update_byte_interval,
        bytes_total,
    );
    let mut output_file_buffered = BufWriter::with_capacity(
        options.buffer_size,
        output_file_progress_monitored,
    );



    let final_number_of_bytes_copied = std::io::copy(
        &mut input_file_buffered,
        &mut output_file_buffered,
    )
    .map_err(|error| FileError::OtherIoError { error })?;



    // Unwrap writers and flush any remaining output.
    let (mut output_file, mut copy_progress, mut progress_handler) = output_file_buffered
        .into_inner()
        .map_err(|error| FileError::OtherIoError {
            error: error.into_error(),
        })?
        .into_inner();

    output_file
        .flush()
        .map_err(|error| FileError::OtherIoError { error })?;

    // Perform one last progress update.
    copy_progress.bytes_finished = final_number_of_bytes_copied;
    progress_handler(&copy_progress);

    Ok(final_number_of_bytes_copied)
}



/// Copy a single file from the `source_file_path` to the `target_file_path` with progress reporting.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
///
/// ## Progress reporting
/// You must also provide a progress handler that receives a
/// [`&FileProgress`][super::FileProgress] containing progress state.
///
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`][FileCopyWithProgressOptions::progress_update_byte_interval] option.
/// This option is the *minumum* amount of bytes written between two progress reports.
/// As such this function does not guarantee a specific amount of progress reports per file size.
/// It does, however, guarantee at least one progress report: the final one, which happens when the file is completely copied.
///
///
/// ## Special return value cases
/// If copying is skipped due to and existing file and
/// [`options.skip_existing`][FileCopyOptions::skip_existing],
/// the return value will be `Ok(0)`.
///
///
/// ## Symbolic links
/// If `source_file_path` is a symbolic link to a file, the contents of the file it points to will be copied to `target_file_path`
/// (same behaviour as `cp` without `-P` on Unix).
///
///
/// ## Internals
/// This function handles copying itself by opening handles of both files itself
/// and buffering reads and writes (see also the [`option.buffer_size`][FileCopyWithProgressOptions::buffer_size] option).
pub fn copy_file_with_progress<P, T, F>(
    source_file_path: P,
    target_file_path: T,
    options: FileCopyWithProgressOptions,
    progress_handler: F,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&FileProgress),
{
    let source_file_path = source_file_path.as_ref();
    let target_file_path = target_file_path.as_ref();

    let ValidatedSourceFilePath {
        source_file_path, ..
    } = validate_source_file_path(source_file_path)?;

    // Ensure the target file path doesn't exist yet
    // (unless `overwrite_existing` is `true`)
    // and that it isn't already a directory path.
    match target_file_path.try_exists() {
        Ok(exists) => {
            if exists {
                // Ensure we don't try to copy the file into itself.
                let canonicalized_source_path = source_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessSourceFile { error })?;
                let canonicalized_target_path = target_file_path
                    .canonicalize()
                    .map_err(|error| FileError::UnableToAccessTargetFile { error })?;

                if canonicalized_source_path.eq(&canonicalized_target_path) {
                    return Err(FileError::SourceAndTargetAreTheSameFile);
                }
            }

            if exists && options.skip_existing {
                return Ok(0);
            }

            if exists && !options.overwrite_existing {
                return Err(FileError::AlreadyExists);
            }
        }
        Err(error) => return Err(FileError::UnableToAccessTargetFile { error }),
    }

    // All checks have passed, we must now copy the file.
    // Unlike in the `copy_file` function, we must copy the file ourselves, as we
    // can't report progress otherwise. This is delegated to the `copy_file_with_progress_unchecked`
    // function which is used in other parts of the library as well.
    copy_file_with_progress_unchecked(
        &source_file_path,
        target_file_path,
        options,
        progress_handler,
    )
}
