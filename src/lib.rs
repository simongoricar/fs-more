//! Convenient file and directory operations built on top of [`std::fs`] with improved error handling.
//! Includes copying or moving files and directories with progress reporting.
//!
//! ## Main features
//! - copying and moving files or directories with in-depth configuration options (including IO buffering settings, copying depth, etc.)
//! - copying and moving files or directories **with progress reporting**, if needed,
//! - scanning directories with depth and other options, and
//! - calculating file or directory sizes.
//!
//! To start off, visit the [`directory`] and [`file`][mod@file] modules
//! for more information and a list of functions.
//!
//!
//! <br>
//!
//! ## Feature flags
//! The following feature flags are available:
//! - `fs-err`: enables the optional [`fs-err`](../fs_err) support, enabling more helpful underlying IO error messages
//!   (though `fs-more` already provides many on its own).
//! - `miette`: derives [`miette::Diagnostic`](../miette/derive.Diagnostic.html) on all
//!   [error types](crate::error),
//!   allowing users to conveniently call e.g. [`wrap_err`](../miette/trait.Context.html#tymethod.wrap_err) on the error.
//!
//! <br>
//!
//! ## Examples
//!
//! Copying a file and getting updates on the progress:
//! ```no_run
//! # use std::path::Path;
//! # use fs_more::error::FileError;
//! # use fs_more::file::FileCopyWithProgressOptions;
//! # fn main() -> Result<(), FileError> {
//! let source_path = Path::new("./source-file.txt");
//! let target_path = Path::new("./target-file.txt");
//!
//! let bytes_copied = fs_more::file::copy_file_with_progress(
//!     source_path,
//!     target_path,
//!     FileCopyWithProgressOptions::default(),
//!     |progress| {
//!         let percent_copied =
//!             (progress.bytes_finished as f64) / (progress.bytes_total as f64)
//!             * 100.0;
//!
//!         println!("Copied {:.2}% of the file!", percent_copied);
//!     }
//! )?;
//!
//! println!("Copied {bytes_copied} bytes!");
//! # Ok(())
//! # }
//! ```
//!
//! Moving a directory and getting updates on the progress:
//! ```no_run
//! # use std::path::Path;
//! # use fs_more::error::DirectoryError;
//! # use fs_more::directory::DirectoryMoveWithProgressOptions;
//! # use fs_more::directory::TargetDirectoryRule;
//! # fn main() -> Result<(), DirectoryError> {
//! let source_path = Path::new("./source-directory");
//! let target_path = Path::new("./target-directory");
//!
//! let moved = fs_more::directory::move_directory_with_progress(
//!     source_path,
//!     target_path,
//!     DirectoryMoveWithProgressOptions {
//!         target_directory_rule: TargetDirectoryRule::AllowEmpty,
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
//!     moved.num_files_moved,
//!     moved.num_directories_moved,
//!     moved.used_strategy
//! );
//! # Ok(())
//! # }
//! ```
//!
//! <br>
//!
//! ## Inspirations
//!
//! <details>
//! <summary>Inspired by <code>fs_extra</code></summary>
//!
//! `fs-more` is very much not a fork, but its API surface has been partially inspired by
//! parts of the [`fs_extra`](https://github.com/webdesus/fs_extra) library - thank you!
//!
//! </details>
//!

#![deny(missing_docs)]

pub mod directory;
pub mod error;
pub mod file;
