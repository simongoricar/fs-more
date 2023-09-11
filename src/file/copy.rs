use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write},
    path::Path,
};

use super::{
    progress::{FileProgress, ProgressWriter},
    validate_source_file_path,
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


/// Copy a single file from the `source_file_path` to the `target_file_path`.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
/// ## Options
/// If `options.overwrite_existing` is `true`, an existing target file will be overwritten if it happens to exist.
///
/// If `options.overwrite_existing` is `false` and the target file exists, this function will
/// return `Err` with [`FileError::AlreadyExists`][crate::error::FileError::AlreadyExists],
/// unless `options.skip_existing` is `true`, in which case `Ok(0)` is returned.
///
/// ## Internals
/// This function internally delegates copying to [`std::fs::copy`] from the standard library (unlike [`copy_file_with_progress`]).
pub fn copy_file<P, T>(
    source_file_path: P,
    target_file_path: T,
    options: &FileCopyOptions,
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
    let num_bytes_copied = std::fs::copy(source_file_path, target_file_path)
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
/// *Warning:* no checks (e.g. whether source exists or whether target is a directory or already exists)
/// are done.
pub(crate) fn copy_file_with_progress_unchecked<F>(
    source_file_path: &Path,
    target_file_path: &Path,
    options: &FileCopyWithProgressOptions,
    progress_handler: F,
) -> Result<u64, FileError>
where
    F: FnMut(&FileProgress),
{
    let bytes_total = source_file_path
        .metadata()
        .map_err(|error| FileError::OtherIoError { error })?
        .len();

    // Open a file for reading and a file for writing,
    // wrap them in buffers and progress monitors, then copy the file.
    let input_file = OpenOptions::new()
        .read(true)
        .open(source_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;

    let mut input_file_buffered =
        BufReader::with_capacity(options.buffer_size, input_file);


    let output_file = OpenOptions::new()
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
    let (mut output_file, mut copy_progress, mut progress_handler) =
        output_file_buffered
            .into_inner()
            .map_err(|error| FileError::OtherIoError {
                error: error.into_error(),
            })?
            .into_inner();

    output_file
        .flush()
        .map_err(|error| FileError::OtherIoError { error })?;

    // Perform one last progress update.
    copy_progress.bytes_copied = final_number_of_bytes_copied;
    progress_handler(&copy_progress);

    Ok(final_number_of_bytes_copied)
}



/// Copy a single file from the `source_file_path` to the `target_file_path`.
///
/// The target path must be the actual target file path and cannot be a directory.
/// Returns the number of bytes moved (i.e. the file size).
///
/// You must also provide a progress handler that receives a
/// [`&FileProgress`][super::FileProgress] on each progress update.
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`][FileCopyWithProgressOptions::progress_update_byte_interval] option.
/// That option is the *minumum* amount of bytes written between two progress reports, meaning we can't guarantee
/// a specific amount of progress reports per file size. We do, however, guarantee at least one progress report (the final one).
///
///
/// ## Options
/// If [`options.overwrite_existing`][FileCopyWithProgressOptions::overwrite_existing] is `true`,
/// an existing target file will be overwritten (if it happens to exist, otherwise the flag is ignored).
///
/// If [`options.overwrite_existing`][FileCopyWithProgressOptions::overwrite_existing] is `false`
/// and the target file exists, this function will return `Err`
/// with [`FileError::AlreadyExists`][crate::error::FileError::AlreadyExists],
/// unless [`options.skip_existing`][FileCopyWithProgressOptions::skip_existing] is `true`,
/// in which case `Ok(0)` is returned.
///
/// ## Internals
/// This function handles copying itself by opening handles of both files itself
/// and buffering reads and writes (see the [`option.buffer_size`][FileCopyWithProgressOptions::buffer_size] option).
pub fn copy_file_with_progress<P, T, F>(
    source_file_path: P,
    target_file_path: T,
    options: &FileCopyWithProgressOptions,
    progress_handler: F,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&FileProgress),
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
        source_file_path,
        target_file_path,
        options,
        progress_handler,
    )
}
