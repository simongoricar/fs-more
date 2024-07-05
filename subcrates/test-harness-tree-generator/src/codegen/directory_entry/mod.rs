mod preparation;
pub use preparation::*;

mod generation;
pub(crate) use generation::*;
use thiserror::Error;

use super::{symlink_entry::SymlinkEntryError, TreeRegistryError};



#[derive(Debug, Error)]
pub enum DirectoryEntryError {
    #[error("symlink entry failed to prepare or generate")]
    SymlinkSubEntryError(
        #[from]
        #[source]
        SymlinkEntryError,
    ),

    #[error("failed to register tree entry into registry")]
    TreeEntryRegistryError(
        #[from]
        #[source]
        TreeRegistryError,
    ),
}
