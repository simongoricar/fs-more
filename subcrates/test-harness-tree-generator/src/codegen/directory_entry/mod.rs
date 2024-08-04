mod preparation;
pub use preparation::*;

mod generation;
pub(crate) use generation::*;
use thiserror::Error;

use super::{
    broken_symlink_entry::BrokenSymlinkEntryError,
    symlink_entry::SymlinkEntryError,
    TreeRegistryError,
};



#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum DirectoryEntryError {
    #[error("symlink entry failed to prepare or generate")]
    SymlinkSubEntryError(
        #[from]
        #[source]
        SymlinkEntryError,
    ),

    #[error("broken symlink entry failed to prepare or generate")]
    BrokenSymlinkSubEntryError(
        #[from]
        #[source]
        BrokenSymlinkEntryError,
    ),

    #[error("failed to register tree entry into registry")]
    TreeEntryRegistryError(
        #[from]
        #[source]
        TreeRegistryError,
    ),
}
