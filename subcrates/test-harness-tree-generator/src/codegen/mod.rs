use std::collections::{hash_map::Entry, HashMap};

use broken_symlink_entry::{GeneratedBrokenSymlinkEntry, PreparedBrokenSymlinkEntry};
use directory_entry::{GeneratedDirectoryEntry, PreparedDirectoryEntry};
use file_entry::{GeneratedFileEntry, PreparedFileEntry};
use proc_macro2::TokenStream;
use symlink_entry::{GeneratedSymlinkEntry, PreparedSymlinkEntry};
use syn::Ident;
use thiserror::Error;

use crate::name_collision::NameCollisionAvoider;

pub mod broken_symlink_entry;
pub mod directory_entry;
pub mod file_entry;
pub mod final_source_file;
pub mod symlink_entry;


#[derive(Clone, Debug)]
pub enum AnyPreparedEntry {
    Directory {
        entry: PreparedDirectoryEntry,
        actual_field_name_on_parent_ident: Ident,
    },
    File {
        entry: PreparedFileEntry,
        actual_field_name_on_parent_ident: Ident,
    },
    Symlink {
        entry: PreparedSymlinkEntry,
        actual_field_name_on_parent_ident: Ident,
    },
    BrokenSymlink {
        entry: PreparedBrokenSymlinkEntry,
        actual_field_name_on_parent_ident: Ident,
    },
}

impl AnyPreparedEntry {
    pub fn actual_field_name_on_parent_ident(&self) -> &Ident {
        match self {
            AnyPreparedEntry::Directory {
                actual_field_name_on_parent_ident,
                ..
            } => actual_field_name_on_parent_ident,
            AnyPreparedEntry::File {
                actual_field_name_on_parent_ident,
                ..
            } => actual_field_name_on_parent_ident,
            AnyPreparedEntry::Symlink {
                actual_field_name_on_parent_ident,
                ..
            } => actual_field_name_on_parent_ident,
            AnyPreparedEntry::BrokenSymlink {
                actual_field_name_on_parent_ident,
                ..
            } => actual_field_name_on_parent_ident,
        }
    }

    pub fn entry_id(&self) -> Option<&String> {
        match self {
            AnyPreparedEntry::Directory { entry, .. } => entry.entry_id.as_ref(),
            AnyPreparedEntry::File { entry, .. } => entry.entry_id.as_ref(),
            AnyPreparedEntry::Symlink { entry, .. } => entry.entry_id.as_ref(),
            AnyPreparedEntry::BrokenSymlink { entry, .. } => entry.entry_id.as_ref(),
        }
    }

    pub fn path_relative_to_harness_root(&self) -> &str {
        match self {
            AnyPreparedEntry::Directory { entry, .. } => {
                entry.directory_path_relative_to_tree_root.as_str()
            }
            AnyPreparedEntry::File { entry, .. } => entry.file_path_relative_to_tree_root.as_str(),
            AnyPreparedEntry::Symlink { entry, .. } => {
                entry.symlink_path_relative_to_tree_root.as_str()
            }
            AnyPreparedEntry::BrokenSymlink { entry, .. } => {
                entry.symlink_path_relative_to_tree_root.as_str()
            }
        }
    }

    pub fn struct_type_ident(&self) -> &Ident {
        match self {
            AnyPreparedEntry::Directory { entry, .. } => &entry.struct_type_ident,
            AnyPreparedEntry::File { entry, .. } => &entry.struct_type_ident,
            AnyPreparedEntry::Symlink { entry, .. } => &entry.struct_type_ident,
            AnyPreparedEntry::BrokenSymlink { entry, .. } => &entry.struct_type_ident,
        }
    }
}



pub(crate) enum AnyGeneratedEntry {
    Directory {
        entry: GeneratedDirectoryEntry,
        actual_field_name_ident_on_parent: Ident,
    },
    File {
        entry: GeneratedFileEntry,
        actual_field_name_ident_on_parent: Ident,
    },
    Symlink {
        entry: GeneratedSymlinkEntry,
        actual_field_name_ident_on_parent: Ident,
    },
    BrokenSymlink {
        entry: GeneratedBrokenSymlinkEntry,
        actual_field_name_ident_on_parent: Ident,
    },
}

impl AnyGeneratedEntry {
    pub fn generated_code(&self) -> &TokenStream {
        match self {
            AnyGeneratedEntry::Directory { entry, .. } => &entry.generated_code,
            AnyGeneratedEntry::File { entry, .. } => &entry.generated_code,
            AnyGeneratedEntry::Symlink { entry, .. } => &entry.generated_code,
            AnyGeneratedEntry::BrokenSymlink { entry, .. } => &entry.generated_code,
        }
    }

    pub fn actual_field_name_ident_on_parent(&self) -> &Ident {
        match self {
            AnyGeneratedEntry::Directory {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
            AnyGeneratedEntry::File {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
            AnyGeneratedEntry::Symlink {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
            AnyGeneratedEntry::BrokenSymlink {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
        }
    }

    pub fn struct_type_ident(&self) -> &Ident {
        match self {
            AnyGeneratedEntry::Directory { entry, .. } => &entry.struct_type_ident,
            AnyGeneratedEntry::File { entry, .. } => &entry.struct_type_ident,
            AnyGeneratedEntry::Symlink { entry, .. } => &entry.struct_type_ident,
            AnyGeneratedEntry::BrokenSymlink { entry, .. } => &entry.struct_type_ident,
        }
    }

    pub fn documentation_for_parent_field(&self) -> &str {
        match self {
            AnyGeneratedEntry::Directory { entry, .. } => &entry.documentation_for_parent_field,
            AnyGeneratedEntry::File { entry, .. } => &entry.documentation_for_parent_field,
            AnyGeneratedEntry::Symlink { entry, .. } => &entry.documentation_for_parent_field,
            AnyGeneratedEntry::BrokenSymlink { entry, .. } => &entry.documentation_for_parent_field,
        }
    }
}



pub struct CodeGenerationContext {
    struct_name_collision_avoider: NameCollisionAvoider,
    prepared_entry_registry: PreparedEntryRegistry,
}



#[derive(Error, Debug)]
pub enum TreeRegistryError {
    #[error("duplicate entry ID in tree: {id}")]
    DuplicateEntryId { id: String },
}


pub struct PreparedEntryRegistry {
    id_to_entry_map: HashMap<String, AnyPreparedEntry>,
}

impl PreparedEntryRegistry {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id_to_entry_map: HashMap::new(),
        }
    }

    pub fn add_prepared_entry(
        &mut self,
        prepared_entry: AnyPreparedEntry,
    ) -> Result<(), TreeRegistryError> {
        let optional_entry_id = prepared_entry.entry_id();

        let Some(entry_id) = optional_entry_id else {
            return Ok(());
        };


        match self.id_to_entry_map.entry(entry_id.to_owned()) {
            Entry::Occupied(_) => Err(TreeRegistryError::DuplicateEntryId {
                id: entry_id.to_owned(),
            }),
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(prepared_entry);

                Ok(())
            }
        }
    }

    pub fn entry_by_id(&self, id: &str) -> Option<&AnyPreparedEntry> {
        self.id_to_entry_map.get(id)
    }
}
