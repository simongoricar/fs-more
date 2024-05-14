use std::{
    io::{BufReader, BufWriter, Write},
    path::Path,
};

use_enabled_fs_module!();

use super::{
    progress::{FileProgress, ProgressWriter},
    validate_destination_file_path,
    validate_source_file_path,
    DestinationValidationAction,
    ExistingFileBehaviour,
    ValidatedDestinationFilePath,
    ValidatedSourceFilePath,
};
use crate::{error::FileError, use_enabled_fs_module};



/// Options that influence the [`copy_file`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyFileOptions {
    /// How to behave for a destination file that already exists.
    pub existing_destination_file_behaviour: ExistingFileBehaviour,
}


#[allow(clippy::derivable_impls)]
impl Default for CopyFileOptions {
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
        }
    }
}


/// Results of a successful file copy operation.
///
/// Returned from: [`copy_file`] and [`copy_file_with_progress`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CopyFileFinished {
    /// The destination file did not exist prior to the operation.
    /// The file was freshly created and written to.
    Created {
        /// Number of bytes written to the file.
        bytes_copied: u64,
    },

    /// The destination file already existed, and was overwritten by the copy operation.
    Overwritten {
        /// Number of bytes written to the file.
        bytes_copied: u64,
    },

    /// The destination file already existed, and the copy operation was skipped.
    ///
    /// This can only be returned when existing destination file behaviour
    /// is set to [`ExistingFileBehaviour::Skip`].
    ///
    ///
    /// [`options.existing_destination_file_behaviour`]: CopyFileOptions::existing_destination_file_behaviour
    Skipped,
}



/// Copies a single file from the source to the destination path.
///
/// The source file path must be an existing file, or a symlink to one.
/// The destination path must be a *file* path, and must not point to a directory.
///
///
/// # Symbolic links
/// Symbolic links are not preserved.
///
/// This means the following: if `source_file_path` leads to a symbolic link that points to a file,
/// the contents of the file at the symlink target will be copied to `destination_file_path`.
///
/// This matches the behaviour of `cp` without `--no-dereference` (`-P`) on Unix[^unix-cp].
///
///
///
/// # Options
/// See [`CopyFileOptions`] for available file copying options.
///
///
/// # Return value
/// If the copy succeeds, the function returns [`CopyFileFinished`],
/// which contains information about whether the file was created,
/// overwritten or skipped. The struct includes the number of bytes copied,
/// if relevant.
///
///
/// # Errors
/// If the file cannot be copied to the destination, a [`FileError`] is returned;
/// see its documentation for more details.
/// Here is a non-exhaustive list of error causes:
/// - If the source path has issues (does not exist, does not have the correct permissions, etc.), one of
///   [`SourceFileNotFound`], [`SourcePathNotAFile`],
///   [`UnableToAccessSourceFile`], or [`UnableToCanonicalizeSourceFilePath`]
///   variants will be returned.
/// - If the destination already exists, and [`options.existing_destination_file_behaviour`]
///   is set to [`ExistingFileBehaviour::Abort`], then a [`DestinationPathAlreadyExists`]
///   will be returned.
/// - If the source and destination paths are canonically actually the same file,
///   then copying will be aborted with [`SourceAndDestinationAreTheSame`].
/// - If the destination path has other issues (is a directory, does not have the correct permissions, etc.),
///   [`UnableToAccessDestinationFile`] or [`UnableToCanonicalizeDestinationFilePath`]
///   will be returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// <br>
///
/// #### See also
/// If you are looking for a file copying function that reports progress,
/// see [`copy_file_with_progress`].
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
/// This function currently delegates file IO to [`std::fs::copy`],
/// or [`fs_err::copy`](https://docs.rs/fs-err/latest/fs_err/fn.copy.html)
/// if the `fs-err` feature flag is enabled.
///
/// </details>
///
///
/// [`options.existing_destination_file_behaviour`]: CopyFileOptions::existing_destination_file_behaviour
/// [`SourceFileNotFound`]: FileError::SourceFileNotFound
/// [`SourcePathNotAFile`]: FileError::SourcePathNotAFile
/// [`UnableToAccessSourceFile`]: FileError::UnableToAccessSourceFile
/// [`UnableToCanonicalizeSourceFilePath`]: FileError::UnableToCanonicalizeSourceFilePath
/// [`DestinationPathAlreadyExists`]: FileError::DestinationPathAlreadyExists
/// [`UnableToAccessDestinationFile`]: FileError::UnableToAccessDestinationFile
/// [`UnableToCanonicalizeDestinationFilePath`]: FileError::UnableToCanonicalizeDestinationFilePath
/// [`SourceAndDestinationAreTheSame`]: FileError::SourceAndDestinationAreTheSame
/// [`OtherIoError`]: FileError::OtherIoError
/// [^unix-cp]: Source for coreutils' `cp` is available
///     [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/cp.c).
pub fn copy_file<S, D>(
    source_file_path: S,
    destination_file_path: D,
    options: CopyFileOptions,
) -> Result<CopyFileFinished, FileError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();


    let validated_source_file_path = validate_source_file_path(source_file_path)?;

    let ValidatedDestinationFilePath {
        destination_file_path,
        exists: destination_file_exists,
    } = match validate_destination_file_path(
        &validated_source_file_path,
        destination_file_path,
        options.existing_destination_file_behaviour,
    )? {
        DestinationValidationAction::Continue(validated_path) => validated_path,
        DestinationValidationAction::SkipCopyOrMove => {
            return Ok(CopyFileFinished::Skipped);
        }
    };

    let ValidatedSourceFilePath {
        source_file_path, ..
    } = validated_source_file_path;


    // All checks have passed, pass the copying onto Rust's standard library.
    // Note that a time-of-check time-of-use errors are certainly possible
    // (hence [`FileError::OtherIoError`], though there may be other reasons for it as well).

    let bytes_copied = fs::copy(source_file_path, destination_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;



    match destination_file_exists {
        true => Ok(CopyFileFinished::Overwritten { bytes_copied }),
        false => Ok(CopyFileFinished::Created { bytes_copied }),
    }
}



/// Options that influence the [`copy_file_with_progress`] function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyFileWithProgressOptions {
    /// How to behave for destination files that already exist.
    pub existing_destination_file_behaviour: ExistingFileBehaviour,

    /// Internal buffer size used for reading the source file.
    ///
    /// Defaults to 64 KiB.
    pub read_buffer_size: usize,

    /// Internal buffer size used for writing to the destination file.
    ///
    /// Defaults to 64 KiB.
    pub write_buffer_size: usize,

    /// The smallest amount of bytes copied between two consecutive progress reports.
    ///
    /// Increase this value to make progress reports less frequent,
    /// and decrease it to make them more frequent. Keep in mind that
    /// decreasing the interval will likely come at some performance cost,
    /// depending on your progress handling closure.
    ///
    /// *Note that this is the minimum interval.* The actual reporting interval can be larger.
    /// Consult [`copy_file_with_progress`] documentation for more details.
    ///
    /// Defaults to 64 KiB.
    pub progress_update_byte_interval: u64,
}

impl Default for CopyFileWithProgressOptions {
    /// Constructs relatively safe defaults for copying a file:
    /// - aborts if there is an existing destination file ([`ExistingFileBehaviour::Abort`]),
    /// - sets buffer size for reading and writing to 64 KiB, and
    /// - sets the progress update closure call interval to 64 KiB.
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            // 64 KiB
            read_buffer_size: 1024 * 64,
            // 64 KiB
            write_buffer_size: 1024 * 64,
            // TODO Increase this to a much larger default value to avoid performance problems.
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Copies the specified file from the source to the destination using the provided options
/// and progress reporting closure.
///
/// This is done by opening two file handles (one for reading, another for writing),
/// wrapping them in buffered readers and writers, plus our progress tracker intermediary,
/// and then finally using the [`std::io::copy`] function to copy the entire file.
///
///
/// # Invariants
/// **Be warned:** no path validation or other checks are performed before copying.
/// It is fully up to the caller to use e.g. [`validate_source_file_path`] +
/// [`validate_destination_file_path`], before passing the validated paths to this function.
pub(crate) fn copy_file_with_progress_unchecked<F>(
    source_file_path: &Path,
    destination_file_path: &Path,
    options: CopyFileWithProgressOptions,
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

    let mut input_file_buffered = BufReader::with_capacity(options.read_buffer_size, input_file);


    let output_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(destination_file_path)
        .map_err(|error| FileError::OtherIoError { error })?;

    let output_file_progress_monitored = ProgressWriter::new(
        output_file,
        progress_handler,
        options.progress_update_byte_interval,
        bytes_total,
    );
    let mut output_file_buffered = BufWriter::with_capacity(
        options.write_buffer_size,
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



/// Copies a single file from the source to the destination path, with progress reporting.
///
/// The source file path must be an existing file, or a symlink to one.
/// The destination path must be a *file* path, and must not point to a directory.
///
///
/// # Symbolic links
/// Symbolic links are not preserved.
///
/// This means the following: if `source_file_path` leads to a symbolic link that points to a file,
/// the contents of the file at the symlink target will be copied to `destination_file_path`.
///
/// This matches the behaviour of `cp` without `--no-dereference` (`-P`) on Unix[^unix-cp].
///
///
/// # Options
/// See [`CopyFileWithProgressOptions`] for available file copying options.
///
///
/// # Return value
/// If the copy succeeds, the function returns [`CopyFileFinished`],
/// which contains information about whether the file was created,
/// overwritten or skipped. The struct includes the number of bytes copied,
/// if relevant.
///
///
/// # Progress reporting
/// This function allows you to receive progress reports by passing
/// a `progress_handler` closure. It will be called with
/// a reference to [`FileProgress`] regularly.
///
/// You can control the progress reporting frequency by setting the
/// [`options.progress_update_byte_interval`] option to a sufficiently small or large value,
/// but note that smaller intervals are likely to have an impact on performance.
/// The value of this option is the minimum amount of bytes written to a file between
/// two calls to the provided `progress_handler`.
///
/// This function does not guarantee a precise amount of progress reports per file size
/// and progress reporting interval setting; it does, however, guarantee at least one progress report:
/// the final one, which happens when the file has been completely copied.
/// In most cases though, the number of calls to
/// the closure will be near the expected amount,
/// which is `file_size / progress_update_byte_interval`.
///
///
/// # Errors
/// If the file cannot be copied to the destination, a [`FileError`] is returned;
/// see its documentation for more details.
/// Here is a non-exhaustive list of error causes:
/// - If the source path has issues (does not exist, does not have the correct permissions, etc.), one of
///   [`SourceFileNotFound`], [`SourcePathNotAFile`],
///   [`UnableToAccessSourceFile`], or [`UnableToCanonicalizeSourceFilePath`]
///   variants will be returned.
/// - If the destination already exists, and [`options.existing_destination_file_behaviour`]
///   is set to [`ExistingFileBehaviour::Abort`], then a [`DestinationPathAlreadyExists`]
///   will be returned.
/// - If the source and destination paths are canonically actually the same file,
///   then copying will be aborted with [`SourceAndDestinationAreTheSame`].
/// - If the destination path has other issues (is a directory, does not have the correct permissions, etc.),
///   [`UnableToAccessDestinationFile`] or [`UnableToCanonicalizeDestinationFilePath`]
///   will be returned.
///
/// There do exist other failure points, mostly due to unavoidable
/// [time-of-check time-of-use](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use)
/// issues and other potential IO errors that can prop up.
/// These errors are grouped under the [`OtherIoError`] variant.
///
///
/// <br>
///
/// #### See also
/// If you are looking for a file copying function that does not report progress,
/// see [`copy_file`].
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
/// Unlike [`copy_file`], this function handles copying itself by opening file handles for
/// both the source and destination file, then buffering reads and writes.
///
/// </details>
///
///
/// [`options.progress_update_byte_interval`]: CopyFileWithProgressOptions::progress_update_byte_interval
/// [`options.existing_destination_file_behaviour`]: CopyFileOptions::existing_destination_file_behaviour
/// [`SourceFileNotFound`]: FileError::SourceFileNotFound
/// [`SourcePathNotAFile`]: FileError::SourcePathNotAFile
/// [`UnableToAccessSourceFile`]: FileError::UnableToAccessSourceFile
/// [`UnableToCanonicalizeSourceFilePath`]: FileError::UnableToCanonicalizeSourceFilePath
/// [`DestinationPathAlreadyExists`]: FileError::DestinationPathAlreadyExists
/// [`UnableToAccessDestinationFile`]: FileError::UnableToAccessDestinationFile
/// [`UnableToCanonicalizeDestinationFilePath`]: FileError::UnableToCanonicalizeDestinationFilePath
/// [`SourceAndDestinationAreTheSame`]: FileError::SourceAndDestinationAreTheSame
/// [`OtherIoError`]: FileError::OtherIoError
/// [^unix-cp]: Source for coreutils' `cp` is available
///     [here](https://github.com/coreutils/coreutils/blob/ccf47cad93bc0b85da0401b0a9d4b652e4c930e4/src/cp.c).
pub fn copy_file_with_progress<P, T, F>(
    source_file_path: P,
    destination_file_path: T,
    options: CopyFileWithProgressOptions,
    progress_handler: F,
) -> Result<CopyFileFinished, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&FileProgress),
{
    let source_file_path = source_file_path.as_ref();
    let destination_file_path = destination_file_path.as_ref();


    let validated_source_file_path = validate_source_file_path(source_file_path)?;

    let ValidatedDestinationFilePath {
        destination_file_path,
        exists: destination_file_exists,
    } = match validate_destination_file_path(
        &validated_source_file_path,
        destination_file_path,
        options.existing_destination_file_behaviour,
    )? {
        DestinationValidationAction::Continue(validated_path) => validated_path,
        DestinationValidationAction::SkipCopyOrMove => {
            return Ok(CopyFileFinished::Skipped);
        }
    };

    let ValidatedSourceFilePath {
        source_file_path, ..
    } = validated_source_file_path;


    // All checks have passed, we must now copy the file.
    // Unlike in the `copy_file` function, we must copy the file ourselves, as we
    // can't report progress otherwise. This is delegated to the `copy_file_with_progress_unchecked`
    // function which is used in other parts of the library as well.

    let bytes_copied = copy_file_with_progress_unchecked(
        &source_file_path,
        &destination_file_path,
        options,
        progress_handler,
    )?;

    match destination_file_exists {
        true => Ok(CopyFileFinished::Overwritten { bytes_copied }),
        false => Ok(CopyFileFinished::Created { bytes_copied }),
    }
}
