#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct SymlinkEntry {
    /// Symlink name (including extension).
    pub name: String,

    /// Optional tree-unique entry ID.
    /// User to refer to entries in the symlink file type, for example.
    pub id: Option<String>,

    /// Entry ID of the destination in the tree (its `id` value).
    pub destination_entry_id: String,
}
