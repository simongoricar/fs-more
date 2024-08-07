#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub enum SymlinkDestinationType {
    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "file"))]
    File,

    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "directory"))]
    Directory,
}


#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct BrokenSymlinkEntry {
    /// Symlink name (including extension).
    pub name: String,

    /// Optional tree-unique entry ID.
    /// User to refer to entries in the symlink file type, for example.
    pub id: Option<String>,

    /// Non-existent destination path, relative to this symlink.
    ///
    /// Destination must not exist.
    pub destination_relative_path: String,

    /// Type of symlink destination.
    ///
    /// This is required on Windows due to different ways of creating
    /// symbolic links depending on the destination type (file or directory).
    pub destination_type: SymlinkDestinationType,
}
