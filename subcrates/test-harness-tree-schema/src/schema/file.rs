#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
#[cfg_attr(feature = "serializable_tree_schema", serde(tag = "type"))]
pub enum FileDataConfiguration {
    /// Creates an empty file.
    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "empty"))]
    Empty,

    /// Creates a file and writes the given `content` into it.
    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "text"))]
    Text { content: String },

    /// Creates a file and seeds it with `file_size_bytes` bytes
    /// of deterministic random data.
    #[cfg_attr(feature = "serializable_tree_schema", serde(rename = "seeded-random"))]
    DeterministicRandom { seed: u64, file_size_bytes: usize },
}



#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_tree_schema",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct FileEntry {
    /// File name (including extension).
    pub name: String,

    /// Optional tree-unique entry ID.
    ///
    /// User to refer to entries in the symlink file type, for example.
    pub id: Option<String>,

    /// Specifies the data to seed this file with.
    /// If `None`, an empty file is created (just like [`FileDataConfiguration::Empty`]).
    pub data: Option<FileDataConfiguration>,
}
