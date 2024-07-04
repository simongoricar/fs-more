use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod file;
pub use file::*;
mod directory;
pub use directory::*;
mod symlink;
pub use symlink::*;




/// Describes an entry in a tree - a file or a directory.
///
/// A directory can additionally contain one or more files
/// or subdirectories.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type")]
pub enum FileSystemHarnessEntry {
    #[serde(rename = "file")]
    File(FileEntry),

    #[serde(rename = "directory")]
    Directory(DirectoryEntry),

    #[serde(rename = "symlink")]
    Symlink(SymlinkEntry),
}



#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct FileSystemHarnessStructure {
    /// A list of hiearhical filesystem entries.
    /// The first level of these entries will reside in the root directory
    /// of the harness.
    pub entries: Vec<FileSystemHarnessEntry>,
}




#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct FileSystemHarnessSchema {
    /// Name of the root struct for the generated filesystem harness.
    /// Will be converted to upper camel case if not already.
    ///
    /// Example: `simple` will generate, among other things, a `Simple` struct,
    /// which will be the root of the harness.
    pub name: String,

    /// File name (without extension) to save the generated harness into.
    ///
    /// Example: `simple` will save the generated harness code into `simple.rs`.
    pub file_name: String,

    /// A short description of the tree.
    pub description: Option<String>,

    /// The full file tree of the harness.
    pub structure: FileSystemHarnessStructure,
}
