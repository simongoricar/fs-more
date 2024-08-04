use std::path::Path;

use fs_more_test_harness_tree_schema::schema::SymlinkEntry;
use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use quote::format_ident;
use syn::Ident;

use super::SymlinkEntryError;
use crate::codegen::CodeGenerationContext;



#[derive(Clone, Debug)]
pub struct PreparedSymlinkEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) symlink_name: String,

    pub(crate) symlink_path_relative_to_tree_root: String,

    pub(crate) symlink_destination_entry_id: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,
}


pub(crate) fn prepare_symlink_entry(
    context: &mut CodeGenerationContext,
    parent_relative_path: &Path,
    symlink: &SymlinkEntry,
) -> Result<PreparedSymlinkEntry, SymlinkEntryError> {
    let friendly_snake_case_field_name = symlink
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_")
        .to_snake_case();

    let preferred_parent_field_ident = format_ident!("{}", friendly_snake_case_field_name);


    let friendly_upper_camel_case_file_name = symlink
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join(" ")
        .to_upper_camel_case();


    let symlink_struct_name = context
        .struct_name_collision_avoider
        .collision_free_name(&friendly_upper_camel_case_file_name);

    let symlink_struct_ident = format_ident!("{}", symlink_struct_name);


    let symlink_relative_path = parent_relative_path.join(&symlink.name);
    let symlink_relative_path_string = symlink_relative_path
        .to_slash()
        .expect("invalid relative file path: not UTF-8!")
        .to_string();


    Ok(PreparedSymlinkEntry {
        entry_id: symlink.id.clone(),
        symlink_name: symlink.name.clone(),
        symlink_path_relative_to_tree_root: symlink_relative_path_string,
        symlink_destination_entry_id: symlink.destination_entry_id.clone(),
        struct_type_ident: symlink_struct_ident,
        preferred_parent_field_ident,
    })
}
