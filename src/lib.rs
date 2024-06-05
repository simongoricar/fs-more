//! Convenient file and directory operations built on top of [`std::fs`] with improved error handling.
//! Includes copying or moving files and directories with progress reporting.
//!
//!
//! # Main features
//! - copy and move files or directories with:
//!     - in-depth configuration options (existing destination file behaviour, IO buffering settings, copying depth, etc.), and
//!     - **progress reporting**, if needed,
//! - scan directories (with options such as scan depth), and
//! - calculate file or directory sizes.
//!
//! <br>
//!
//! To start off, visit the [`directory`] and [`file`][mod@file] modules
//! for more information and a list of functions.
//!
//!
//! <br>
//!
//! # Feature flags
//! The following feature flags enable optional functionality:
//! - `dunce` (*enabled by default*): enables the optional [`dunce`](../dunce/index.html) support:
//!   This automatically strips Windows' UNC paths if they can be represented
//!   using the usual type of path (e.g. `\\?\C:\foo -> C:\foo`) both internally
//!   and in e.g. `DirectoryScan`'s file and directory paths (this is recommended because path canonicalization
//!   very commonly returns UNC paths).
//!   This only has an effect when compiling for Windows targets.
//! - `fs-err` (*disabled by default*): enables the optional [`fs-err`](../fs_err/index.html) support.
//!   While `fs-more` already provides quite extensive [error types](crate::error),
//!   this does enable more helpful error messages for underlying IO errors.
//! - `miette` (*disabled by default*): derives [`miette::Diagnostic`](../miette/derive.Diagnostic.html) on all
//!   [error types](crate::error), allowing users to conveniently
//!   use e.g. [`wrap_err`](../miette/trait.Context.html#tymethod.wrap_err) on the errors returned by this crate.
//!
//! // TODO update feature flags: camino has been added (but is not implemented yet)
//! // TODO either use advanced miette diagnostic features, or remove the feature flag
//! // TODO (also update this in the README)
//!
//! <br>
//!
//! # Examples
//!
//! Copying a file and getting updates on the progress:
//! ```no_run
//! # use std::path::Path;
//! # use fs_more::error::FileError;
//! # use fs_more::file::CopyFileWithProgressOptions;
//! # use fs_more::file::CopyFileFinished;
//! # use fs_more::file::ExistingFileBehaviour;
//! # fn main() -> Result<(), FileError> {
//! let source_path = Path::new("./source-file.txt");
//! let destination_path = Path::new("./target-file.txt");
//!
//! let finished_copy = fs_more::file::copy_file_with_progress(
//!     source_path,
//!     destination_path,
//!     CopyFileWithProgressOptions {
//!         existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
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
//!     CopyFileFinished::Created { bytes_copied } => {
//!         println!("Copied {bytes_copied} bytes!");
//!     }
//!     CopyFileFinished::Overwritten { bytes_copied } => {
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
//! # use fs_more::directory::MoveDirectoryWithProgressOptions;
//! # use fs_more::directory::DestinationDirectoryRule;
//! # fn main() -> Result<(), MoveDirectoryError> {
//! let source_path = Path::new("./source-directory");
//! let destination_path = Path::new("./target-directory");
//!
//! let moved = fs_more::directory::move_directory_with_progress(
//!     source_path,
//!     destination_path,
//!     MoveDirectoryWithProgressOptions {
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

#![warn(missing_docs)]

pub mod directory;
pub mod error;
pub mod file;
mod macros;
