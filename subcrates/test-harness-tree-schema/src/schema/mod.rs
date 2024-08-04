mod file;
pub use file::*;
mod directory;
pub use directory::*;
mod symlink;
pub use symlink::*;
mod broken_symlink;
pub use broken_symlink::*;



/// Describes an entry in a tree - a file or a directory.
///
/// A directory can additionally contain one or more files
/// or subdirectories.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
#[cfg_attr(feature = "serializable_tree_schema", serde(tag = "type"))]
pub enum FileSystemHarnessEntry {
    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "file"))]
    File(FileEntry),

    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "directory"))]
    Directory(DirectoryEntry),

    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "symlink"))]
    Symlink(SymlinkEntry),

    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "broken-symlink"))]
    BrokenSymlink(BrokenSymlinkEntry),
}



#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct FileSystemHarnessStructure {
    /// A list of hiearhical filesystem entries.
    /// The first level of these entries will reside in the root directory
    /// of the harness.
    pub entries: Vec<FileSystemHarnessEntry>,
}




#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
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
