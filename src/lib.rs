//! Convenient file and directory operations built on top of [`std::fs`] with improved error handling.
//! Includes copying / moving files and directories with progress.
//!
//! ## Main features
//! - copying and moving files or directories with in-depth configuration options (including IO buffering settings, copying depth, etc.)
//! - copying and moving files (or directories) **with progress reporting**,
//! - scanning directories with depth and other options, and
//! - calculating file or directory sizes.
//!
//! To start off, visit the [`directory`][crate::directory] and [`file`][crate::file] modules
//! for more information and a list of functions.
//!
//!
//! <br>
//!
//! ## Feature flags
//! The following feature flags are available:
//! - `fs-err`: enables the optional [`fs-err`](../fs_err) support, enabling more helpful underlying IO error messages
//!   (though `fs-more` explicitly provides many on its own).
//!
//! <br>
//!
//! ## Attribution
//!
//! <details>
//! <summary>Inspired by <code>fs_extra</code></summary>
//!
//! `fs-more` isn't a fork, but has been inspired by
//! some of the functionalities of the [`fs_extra`](https://github.com/webdesus/fs_extra) library (thank you!).
//!
//! </details>
//!

#![deny(missing_docs)]

pub mod directory;
pub mod error;
pub mod file;
