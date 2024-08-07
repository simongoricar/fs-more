//! Convenient file and directory operations built on top of [`std::fs`] with improved error handling and in-depth configuration.
//! Includes copying or moving files and directories with progress reporting.
//!
//!
//! # Main features
//! - copy and move files or directories with:
//!     - in-depth configuration options (existing destination file behaviour, IO buffering settings, copying depth, etc.), and
//!     - **progress reporting**, if needed,
//! - scan directories (with options such as scan depth and symlink behaviour), and
//! - calculate file or directory sizes.
//!
//! <br>
//!
//! Visit the **[`directory`]** and **[`file`][mod@file]** modules
//! for a deeper overview of the available features.
//!
//!
//! <br>
//!
//! # Feature flags
//! The following feature flags contain optional functionality:
//!
//! <table>
//!  <thead style="background-color: rgba(0, 0, 0, 0.18)">
//!   <tr>
//!    <th style="text-align:left">
//!
//! **`dunce`**
//! <span style="font-weight: normal">&nbsp;(<i>enabled</i> by default)</span>
//!    </th>
//!   </tr>
//!  </thead>
//!  <tbody>
//!   <tr>
//!    <td>
//!
//! Enables the optional support for [`dunce`](https://docs.rs/dunce) which automatically strips Windows' UNC paths
//! if they can be represented as non-UNC paths (e.g., `\\?\C:\foo` as `C:\foo`). This is done both
//! internally and in external results from e.g., [`DirectoryScanner`].
//!
//! This feature is enabled by default and recommended because path canonicalization on Windows very commonly returns UNC paths.
//! `dunce` only has an effect when compiling for Windows targets.
//!    </td>
//!   </tr>
//!  </tbody>
//! </table>
//!    
//!
//! <table>
//!  <thead style="background-color: rgba(0, 0, 0, 0.18)">
//!   <tr>
//!    <th style="text-align:left">
//!
//! **`fs-err`**
//! <span style="font-weight: normal">&nbsp;(disabled by default)</span>
//!    </th>
//!   </tr>
//!  </thead>
//!  <tbody>
//!   <tr>
//!    <td>
//!
//! Enables the optional support for [`fs-err`](https://docs.rs/fs-err) which provides more helpful
//! error messages for underlying IO errors. It should be noted that `fs-more` does already provide plenty
//! of context on errors by itself, which is why this is disabled by default.
//!    </td>
//!   </tr>
//!  </tbody>
//! </table>
//!
//!
//! <br>
//!
//! # Examples
//!
//! Copying a file and getting updates on the progress:
//! ```no_run
//! # use std::path::Path;
//! # use fs_more::error::FileError;
//! # use fs_more::file::FileCopyWithProgressOptions;
//! # use fs_more::file::FileCopyFinished;
//! # use fs_more::file::CollidingFileBehaviour;
//! # fn main() -> Result<(), FileError> {
//! let source_path = Path::new("./source-file.txt");
//! let destination_path = Path::new("./target-file.txt");
//!
//! let finished_copy = fs_more::file::copy_file_with_progress(
//!     source_path,
//!     destination_path,
//!     FileCopyWithProgressOptions {
//!         colliding_file_behaviour: CollidingFileBehaviour::Abort,
//!         ..Default::default()
//!     },
//!     |progress| {
//!         let percent_copied =
//!             (progress.bytes_finished as f64) / (progress.bytes_total as f64)
//!             * 100.0;
//!
//!         println!("Copied {:.2}% of the file!", percent_copied);
//!     }
//! )?;
//!
//! match finished_copy {
//!     FileCopyFinished::Created { bytes_copied } => {
//!         println!("Copied {bytes_copied} bytes!");
//!     }
//!     FileCopyFinished::Overwritten { bytes_copied } => {
//!         println!("Copied {bytes_copied} bytes over an existing file!");
//!     }
//!     // ...
//!     _ => {}
//! };
//!
//! # Ok(())
//! # }
//! ```
//!
//! Moving a directory and getting updates on the progress:
//! ```no_run
//! # use std::path::Path;
//! # use fs_more::error::MoveDirectoryError;
//! # use fs_more::directory::DirectoryMoveWithProgressOptions;
//! # use fs_more::directory::DestinationDirectoryRule;
//! # fn main() -> Result<(), MoveDirectoryError> {
//! let source_path = Path::new("./source-directory");
//! let destination_path = Path::new("./target-directory");
//!
//! let moved = fs_more::directory::move_directory_with_progress(
//!     source_path,
//!     destination_path,
//!     DirectoryMoveWithProgressOptions {
//!         destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
//!         ..Default::default()
//!     },
//!     |progress| {
//!         let percent_moved =
//!             (progress.bytes_finished as f64) / (progress.bytes_total as f64)
//!             * 100.0;
//!
//!         println!(
//!             "Moved {:.2}% of the directory ({} files and {} directories so far).",
//!             percent_moved,
//!             progress.files_moved,
//!             progress.directories_created
//!         );
//!     }
//! )?;
//!
//! println!(
//!     "Moved {} bytes ({} files, {} directories)! Underlying strategy: {:?}.",
//!     moved.total_bytes_moved,
//!     moved.files_moved,
//!     moved.directories_moved,
//!     moved.strategy_used
//! );
//! # Ok(())
//! # }
//! ```
//!
//! [`DirectoryScanner`]: crate::directory::DirectoryScanner

#![warn(missing_docs)]


/// This brings in the README's doctests (and is present only when testing).
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;



/// Default file read buffer size, used as a default in progress tracking functions.
/// Currently equals 64 KiB.
///
/// See also:
/// - [`FileCopyWithProgressOptions`][crate::file::FileCopyWithProgressOptions]
/// - [`FileMoveWithProgressOptions`][crate::file::FileMoveWithProgressOptions]
/// - [`DirectoryCopyWithProgressOptions`][crate::directory::DirectoryCopyWithProgressOptions]
/// - [`DirectoryMoveWithProgressOptions`][crate::directory::DirectoryMoveWithProgressOptions]
const DEFAULT_READ_BUFFER_SIZE: usize = 1024 * 64;


/// Default file write buffer size, used as a default in progress tracking functions.
/// Currently equals 64 KiB.
///
/// See also:
/// - [`FileCopyWithProgressOptions`][crate::file::FileCopyWithProgressOptions]
/// - [`FileMoveWithProgressOptions`][crate::file::FileMoveWithProgressOptions]
/// - [`DirectoryCopyWithProgressOptions`][crate::directory::DirectoryCopyWithProgressOptions]
/// - [`DirectoryMoveWithProgressOptions`][crate::directory::DirectoryMoveWithProgressOptions]
const DEFAULT_WRITE_BUFFER_SIZE: usize = 1024 * 64;


/// Default progress reporting interval, used as a default in progress tracking functions.
/// Currently equals 512 KiB.
///
/// See also:
/// - [`FileCopyWithProgressOptions`][crate::file::FileCopyWithProgressOptions]
/// - [`FileMoveWithProgressOptions`][crate::file::FileMoveWithProgressOptions]
/// - [`DirectoryCopyWithProgressOptions`][crate::directory::DirectoryCopyWithProgressOptions]
/// - [`DirectoryMoveWithProgressOptions`][crate::directory::DirectoryMoveWithProgressOptions]
const DEFAULT_PROGRESS_UPDATE_BYTE_INTERVAL: u64 = 1024 * 512;

#[macro_use]
mod macros;


pub mod directory;
pub mod error;
pub mod file;
