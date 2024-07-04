use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::FileSystemHarnessEntry;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct DirectoryEntry {
    /// Directory name.
    pub name: String,

    /// Optional tree-unique entry ID.
    ///
    /// User to refer to entries in the symlink file type, for example.
    pub id: Option<String>,

    /// If any, this specifies files and subdirectories
    /// inside this directory.
    pub entries: Option<Vec<FileSystemHarnessEntry>>,
}
