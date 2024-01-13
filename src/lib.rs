//! Convenient Rust file and directory operations.
//!
//! Main features include:
//! - scanning directories,
//! - calculating file or directory sizes,
//! - copying or moving files or directories including in-depth configuration options,
//! - copying or moving files or directories **with progress reporting**,
//!
//! and more filesystem-oriented tools that [`std::fs`] doesn't provide.
//!
//!
//! ### Attribution
//!
//! <details>
//! <summary>Inspired by <code>fs_extra</code></summary>
//!
//! `fs-more` isn't quite a fork, but has been inspired by
//! the [`fs_extra`](https://github.com/webdesus/fs_extra) library (thank you!).
//!
//! </details>
//!

pub mod directory;
pub mod error;
pub mod file;
