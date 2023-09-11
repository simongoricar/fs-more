use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write},
    path::Path,
};

use super::validate_source_file_path;
use crate::error::FileError;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileCopyOptions {
    /// Whether to overwrite an existing target file if it exists already.
    ///
    /// Note that this has lower precedence than `skip_existing`.
    pub overwrite_existing: bool,

    /// Whether to skip copying the file if it already exists.
    /// This takes precedence over `overwrite_existing`.
    pub skip_existing: bool,
}


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


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileCopyWithProgressOptions {
    /// Whether to overwrite an existing target file if it exists already.
    ///
    /// Note that this has lower precedence than `skip_existing`.
    pub overwrite_existing: bool,

    /// Whether to skip copying the file if it already exists.
    /// This takes precedence over `overwrite_existing`.
    pub skip_existing: bool,

    /// Internal buffer size when copying the file, defaults to 64 KiB.
    pub buffer_size: usize,

    /// *Minimum* amount of bytes written between two consecutive progress reports.
    /// Defaults to 64 KiB.
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


#[derive(Clone, PartialEq, Eq)]
pub struct FileCopyProgress {
    pub bytes_copied: u64,
    pub bytes_total: u64,
}

struct ProgressWriter<W: Write, F: FnMut(&FileCopyProgress)> {
    progress: FileCopyProgress,
    inner: W,

    progress_update_byte_interval: u64,
    bytes_written_since_last_progress_update: u64,

    handler: F,
}

impl<W: Write, F: FnMut(&FileCopyProgress)> ProgressWriter<W, F> {
    pub fn new(
        inner: W,
        handler: F,
        progress_update_byte_interval: u64,
        bytes_total: u64,
    ) -> Self {
        Self {
            progress: FileCopyProgress {
                bytes_copied: 0,
                bytes_total,
            },
            inner,
            progress_update_byte_interval,
            bytes_written_since_last_progress_update: 0,
            handler,
        }
    }

    pub fn into_inner(self) -> (W, FileCopyProgress, F) {
        (self.inner, self.progress, self.handler)
    }
}

impl<W: Write, F: FnMut(&FileCopyProgress)> Write for ProgressWriter<W, F> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let inner_write_result = self.inner.write(buf);

        if let Ok(bytes_written) = &inner_write_result {
            self.progress.bytes_copied += *bytes_written as u64;
            self.bytes_written_since_last_progress_update +=
                *bytes_written as u64;
        }

        if self.bytes_written_since_last_progress_update
            > self.progress_update_byte_interval
        {
            (self.handler)(&self.progress);
            self.bytes_written_since_last_progress_update = 0;
        }

        inner_write_result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}


pub fn copy_file_with_progress<P, T, F>(
    source_file_path: P,
    target_file_path: T,
    options: &FileCopyWithProgressOptions,
    progress_handler: F,
) -> Result<u64, FileError>
where
    P: AsRef<Path>,
    T: AsRef<Path>,
    F: FnMut(&FileCopyProgress),
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
    // can't report progress otherwise.
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
