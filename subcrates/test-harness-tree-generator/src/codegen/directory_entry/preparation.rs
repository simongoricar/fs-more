use std::path::Path;

use fs_more_test_harness_tree_schema::schema::{DirectoryEntry, FileSystemHarnessEntry};
use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use quote::format_ident;
use syn::Ident;

use super::DirectoryEntryError;
use crate::{
    codegen::{
        file_entry::prepare_file_entry,
        symlink_entry::prepare_symlink_entry,
        AnyPreparedEntry,
        CodeGenerationContext,
    },
    name_collision::NameCollisionAvoider,
};



#[derive(Clone, Debug)]
pub struct PreparedDirectoryEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) directory_name: String,

    pub(crate) directory_path_relative_to_tree_root: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,

    pub(crate) entries: Vec<AnyPreparedEntry>,
}

impl PreparedDirectoryEntry {
    pub(crate) fn requires_post_initialization_call(&self) -> bool {
        self.entries.iter().any(|entry| match entry {
            AnyPreparedEntry::Directory { entry, .. } => entry.requires_post_initialization_call(),
            AnyPreparedEntry::Symlink { .. } => true,
            AnyPreparedEntry::File { .. } => false,
        })
    }
}



pub(crate) fn prepare_directory_entry(
    context: &mut CodeGenerationContext,
    parent_relative_path: &Path,
    directory: &DirectoryEntry,
) -> Result<PreparedDirectoryEntry, DirectoryEntryError> {
    let friendly_upper_camel_case_directory_name = directory
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join("");

    let directory_struct_name = context.struct_name_collision_avoider.collision_free_name(
        &friendly_upper_camel_case_directory_name
            .as_str()
            .to_upper_camel_case(),
    );

    let directory_struct_name_ident = format_ident!("{}", directory_struct_name);


    let friendly_snake_case_directory_name = directory
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_");

    let preferred_field_name_on_parent = format_ident!("{}", friendly_snake_case_directory_name);


    let directory_relative_path = parent_relative_path.join(&directory.name);
    let directory_relative_path_string = directory_relative_path
        .to_slash()
        .expect("invalid relative directory path: not UTF-8!")
        .to_string();


    let unparsed_directory_entries = directory.entries.to_owned().unwrap_or_default();


    let mut struct_field_name_collision_avoider = NameCollisionAvoider::new_empty();

    let mut prepared_entries = Vec::with_capacity(unparsed_directory_entries.len());
    for entry in &unparsed_directory_entries {
        match entry {
            FileSystemHarnessEntry::File(file_entry) => {
                let prepared_file_entry =
                    prepare_file_entry(context, &directory_relative_path, file_entry);

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_file_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::File {
                    entry: prepared_file_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Directory(directory_entry) => {
                let prepared_directory_entry =
                    prepare_directory_entry(context, &directory_relative_path, directory_entry)?;

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_directory_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Directory {
                    entry: prepared_directory_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Symlink(symlink_entry) => {
                let prepared_symlink_entry =
                    prepare_symlink_entry(context, &directory_relative_path, symlink_entry)?;

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_symlink_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Symlink {
                    entry: prepared_symlink_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
        }
    }


    Ok(PreparedDirectoryEntry {
        entry_id: directory.id.to_owned(),
        directory_name: directory.name.to_owned(),
        directory_path_relative_to_tree_root: directory_relative_path_string,
        struct_type_ident: directory_struct_name_ident,
        preferred_parent_field_ident: preferred_field_name_on_parent,
        entries: prepared_entries,
    })
}
