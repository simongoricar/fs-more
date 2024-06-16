//! Directory copying, moving, scanning and sizing operations.
//! *Includes progress monitoring variants.*
//!
//! <br>
//!
//! ##### Feature Overview
//!
//! | | <span style="font-weight:normal"><i>configured by</i></span> | <span style="font-weight:normal"><i>returns</i></span>
//! |-----------------------------|---------------------------------|:--------------------:|
//! | [`copy_directory`]               | [`DirectoryCopyOptions`]             | [`DirectoryCopyFinished`] <br><sup style="text-align: right">(or [`CopyDirectoryError`])</sup> |
//! | [`copy_directory_with_progress`] | [`DirectoryCopyWithProgressOptions`] | [`DirectoryCopyFinished`] <br><sup style="text-align: right">(or [`CopyDirectoryError`])</sup> |
//! | [`move_directory`]               | [`DirectoryCopyOptions`]             | [`DirectoryMoveFinished`] <br><sup style="text-align: right">(or [`MoveDirectoryError`])</sup> |
//! | [`move_directory_with_progress`] | [`DirectoryCopyOptions`]             | [`DirectoryMoveFinished`] <br><sup style="text-align: right">(or [`MoveDirectoryError`])</sup> |
//! | [`DirectoryScan::scan_with_options`] | [`DirectoryScanOptions`]         | [`DirectoryScan`] <br><sup style="text-align: right">(or [`DirectoryScanError`])</sup> |
//! | [`directory_size_in_bytes`]      | *individual arguments*               | [`u64`] <br><sup style="text-align: right">(or [`DirectorySizeScanError`])</sup> |
//! | [`is_directory_empty`]           |                                      | [`bool`] <br><sup style="text-align: right">(or [`IsDirectoryEmptyError`])</sup> |
//!
//!
//! [`CopyDirectoryError`]: crate::error::CopyDirectoryError
//! [`MoveDirectoryError`]: crate::error::MoveDirectoryError
//! [`DirectorySizeScanError`]: crate::error::DirectorySizeScanError
//! [`IsDirectoryEmptyError`]: crate::error::IsDirectoryEmptyError
//! [`DirectoryScanError`]: crate::error::DirectoryScanError


mod common;
mod copy;
mod r#move;
mod prepared;
mod scan;
mod size;


pub use common::*;
pub use copy::*;
pub use r#move::*;
pub use scan::*;
pub use size::*;
