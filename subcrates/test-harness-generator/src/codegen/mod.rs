use std::collections::{hash_map::Entry, HashMap};

use directory_in_tree::PreparedDirectoryEntry;
use file_in_tree::PreparedFileEntry;
use symlink_in_tree::PreparedSymlinkEntry;
use syn::Ident;
use thiserror::Error;

use crate::name_collision::NameCollisionAvoider;

pub mod directory_in_tree;
pub mod file_in_tree;
pub mod final_source_file;
pub mod symlink_in_tree;


#[derive(Clone, Debug)]
pub enum AnyPreparedEntry {
    Directory {
        entry: PreparedDirectoryEntry,
        actual_field_name_ident_on_parent: Ident,
    },
    File {
        entry: PreparedFileEntry,
        actual_field_name_ident_on_parent: Ident,
    },
    Symlink {
        entry: PreparedSymlinkEntry,
        actual_field_name_ident_on_parent: Ident,
    },
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
        let optional_entry_id = match &prepared_entry {
            AnyPreparedEntry::Directory { entry, .. } => entry.entry_id.as_ref(),
            AnyPreparedEntry::File { entry, .. } => entry.entry_id.as_ref(),
            AnyPreparedEntry::Symlink { entry, .. } => entry.entry_id.as_ref(),
        };

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
