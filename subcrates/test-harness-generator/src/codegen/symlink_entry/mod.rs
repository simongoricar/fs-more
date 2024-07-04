mod preparation;
use std::path::PathBuf;

pub use preparation::*;
mod generation;
pub(crate) use generation::*;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum SymlinkEntryError {
    #[error("unrecognized symlink destination entry ID: {id}")]
    UnrecognizedDestinationId { id: String },

    #[error(
        "chaining symlinks is currently not supported \
        (tried to chain from {} to {})",
        .from.display(),
        .to.display()
    )]
    ChainingSymlinksNotSupported { from: PathBuf, to: PathBuf },
}
