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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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


/// Information about a successful file copy operation.
///
/// See also: [`copy_file`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CopyFileFinished {
    /// The destination file did not exist prior to the operation,
    /// and was freshly created and written to.
    Created {
        /// Number of bytes written to the file.
        bytes_copied: u64,
    },

    /// The destination file already existed, and was overwritten.
    Overwritten {
        /// Number of bytes written to the file.
        bytes_copied: u64,
    },

    /// The destination file already existed, and the copy operation was skipped.
    ///
    /// This variant can be returned when [`options.existing_destination_file_behaviour`]
    /// is set to [`ExistingFileBehaviour::Skip`].
    ///
    ///
    /// [`options.existing_destination_file_behaviour`]: [CopyFileOptions::existing_destination_file_behaviour]
    Skipped,
}



/// Copies a single file from the source to the destination path.
///
/// The destination path must be a *file* path and cannot point to a directory.
///
///
/// ## Return value
/// The function returns [`CopyFileFinished`], which indicates whether the file was created,
/// overwritten or skipped, and includes the number of bytes copied, if relevant.
///
///
/// ## Symbolic links
/// If `source_file_path` leads to a symbolic link to a file,
/// the contents of the link destination file will be copied to `destination_file_path`.
/// This matches the behaviour of `cp` without `-P` on Unix.
///
///
/// ## Implementation
/// This function internally delegates copying to [`std::fs::copy`]
/// (but note that [`copy_file_with_progress`] does not).
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
        DestinationValidationAction::SkipCopy => {
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

    /// The smallest amount of bytes processed between two consecutive progress reports.
    ///
    /// Increase this value to make progress reports less frequent, and decrease it
    /// to make them more frequent.
    ///
    /// *Note that this is the minimum;* the real reporting interval can be larger.
    /// Consult [`copy_file_with_progress`] documentation for more details.
    ///
    /// Defaults to 64 KiB.
    pub progress_update_byte_interval: u64,
}

impl Default for CopyFileWithProgressOptions {
    /// Constructs relatively safe defaults for copying a file:
    /// - aborts if there is an existing destination file,
    /// - sets buffer sizes to 64 KiB, and
    /// - sets the progress update interval to 64 KiB.
    fn default() -> Self {
        Self {
            existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
            // 64 KiB
            read_buffer_size: 1024 * 64,
            // 64 KiB
            write_buffer_size: 1024 * 64,
            // 64 KiB
            progress_update_byte_interval: 1024 * 64,
        }
    }
}


/// Copies the specified file from the source to the destination using the provided options
/// and progress handler.
///
/// This is done by opening two file handles (one for reading, another for writing),
/// wrapping them in buffered readers or writers (and our progress tracker intermediary),
/// and then using the [`std::io::copy`] function to copy the entire file.
///
/// **Be warned:** no checks are performed before copying
/// (e.g. whether source exists or whether target is a directory or already exists).
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



/// Copies a single file from the source to the destination path (with progress reporting).
///
/// The destination path must be a *file* path and cannot point to a directory.
///
///
/// ## Return value
/// The function returns [`CopyFileFinished`], which indicates whether the file was created,
/// overwritten or skipped, and includes the number of bytes copied, if relevant.
///
///
/// ## Progress reporting
/// You must also provide a progress handler that receives a
/// [`&FileProgress`][super::FileProgress] containing progress state.
///
/// You can control the progress update frequency with the
/// [`options.progress_update_byte_interval`] option.
/// This option is the *minumum* amount of bytes written between two progress reports.
/// As such this function does not guarantee a specific amount of progress reports per file size.
/// It does, however, guarantee at least one progress report: the final one, which happens when the file is completely copied.
///
///
/// ## Special return values
/// If [`options.existing_destination_file_behaviour`]
/// is set to [`ExistingFileBehaviour::Skip`], and copying the file is consequently skipped
/// due to it already existing, the return value of this function will be `Ok(0)`.
///
///
/// ## Symbolic links
/// If `source_file_path` leads to a symbolic link to a file,
/// the contents of the file the path points to will be copied to `destination_file_path`.
/// This matches the behaviour of `cp` without `-P` on Unix.s
///
///
/// ## Internals
/// This function handles copying itself by opening handles of both files itself
/// and buffering reads and writes.
///
///
/// [`options.progress_update_byte_interval`]: [CopyFileWithProgressOptions::progress_update_byte_interval]
/// [`options.existing_destination_file_behaviour`]: [CopyFileWithProgressOptions::existing_destination_file_behaviour]
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
        DestinationValidationAction::SkipCopy => {
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
