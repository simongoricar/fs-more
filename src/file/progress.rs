use std::io::Write;

/// File copying or moving progress.
#[derive(Clone, PartialEq, Eq)]
pub struct FileProgress {
    /// Current number of bytes copied or moved to the destination.
    pub bytes_finished: u64,

    /// Total number of bytes that must be copied or moved to the destination.
    pub bytes_total: u64,
}


/// A file write progress handler that implements `Write` and just passes data through.
pub struct ProgressWriter<W: Write, F: FnMut(&FileProgress)> {
    /// Current file copying or moving progress.
    progress: FileProgress,

    /// The inner writer.
    inner: W,

    /// *Minimum* number of bytes required between two progress reports.
    progress_report_byte_interval: u64,

    /// Current number of bytes written since last progress report.
    bytes_written_since_last_progress_report: u64,

    /// Progress report handler.
    handler: F,
}

impl<W: Write, F: FnMut(&FileProgress)> ProgressWriter<W, F> {
    /// Initialize a new `ProgressWriter` by providing a writer, your progress handler,
    /// the minimum amount of bytes written between two progress reports and the total file size in bytes.
    pub fn new(
        inner: W,
        handler: F,
        progress_update_byte_interval: u64,
        bytes_total: u64,
    ) -> Self {
        Self {
            progress: FileProgress {
                bytes_finished: 0,
                bytes_total,
            },
            inner,
            progress_report_byte_interval: progress_update_byte_interval,
            bytes_written_since_last_progress_report: 0,
            handler,
        }
    }

    /// Consumes `self` and returns the inner writer, the last known progress and the progress report closure.
    pub fn into_inner(self) -> (W, FileProgress, F) {
        (self.inner, self.progress, self.handler)
    }
}

impl<W: Write, F: FnMut(&FileProgress)> Write for ProgressWriter<W, F> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let inner_write_result = self.inner.write(buf);

        if let Ok(bytes_written) = &inner_write_result {
            self.progress.bytes_finished += *bytes_written as u64;
            self.bytes_written_since_last_progress_report +=
                *bytes_written as u64;
        }

        if self.bytes_written_since_last_progress_report
            > self.progress_report_byte_interval
        {
            (self.handler)(&self.progress);
            self.bytes_written_since_last_progress_report = 0;
        }

        inner_write_result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
