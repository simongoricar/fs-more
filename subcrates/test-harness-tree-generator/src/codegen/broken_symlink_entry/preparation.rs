use std::path::Path;

use fs_more_test_harness_tree_schema::schema::{BrokenSymlinkEntry, SymlinkDestinationType};
use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use quote::format_ident;
use syn::Ident;

use super::BrokenSymlinkEntryError;
use crate::codegen::CodeGenerationContext;


#[derive(Clone, Debug)]
pub struct PreparedBrokenSymlinkEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) symlink_name: String,

    pub(crate) symlink_path_relative_to_tree_root: String,

    /// Non-existent destination path, relative to this symlink.
    ///
    /// Destination must not exist.
    pub(crate) symlink_destination_relative_path: String,

    pub(crate) symlink_destination_type: SymlinkDestinationType,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,
}



pub(crate) fn prepare_broken_symlink_entry(
    context: &mut CodeGenerationContext,
    parent_relative_path: &Path,
    broken_symlink: &BrokenSymlinkEntry,
) -> Result<PreparedBrokenSymlinkEntry, BrokenSymlinkEntryError> {
    let friendly_snake_case_field_name = broken_symlink
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_")
        .to_snake_case();

    let preferred_parent_field_ident = format_ident!("{}", friendly_snake_case_field_name);


    let friendly_upper_camel_case_file_name = broken_symlink
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join(" ")
        .to_upper_camel_case();


    let broken_symlink_struct_name = context
        .struct_name_collision_avoider
        .collision_free_name(&friendly_upper_camel_case_file_name);

    let broken_symlink_struct_ident = format_ident!("{}", broken_symlink_struct_name);


    let broken_symlink_relative_path = parent_relative_path.join(&broken_symlink.name);
    let broken_symlink_relative_path_string = broken_symlink_relative_path
        .to_slash()
        .expect("invalid relative file path: not UTF-8")
        .to_string();



    Ok(PreparedBrokenSymlinkEntry {
        entry_id: broken_symlink.id.clone(),
        symlink_name: broken_symlink.name.clone(),
        symlink_path_relative_to_tree_root: broken_symlink_relative_path_string,
        symlink_destination_relative_path: broken_symlink.destination_relative_path.clone(),
        symlink_destination_type: broken_symlink.destination_type,
        struct_type_ident: broken_symlink_struct_ident,
        preferred_parent_field_ident,
    })
}
