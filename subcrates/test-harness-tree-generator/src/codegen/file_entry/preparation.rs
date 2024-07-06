use std::path::Path;

use fs_more_test_harness_tree_schema::schema::{FileDataConfiguration, FileEntry};
use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use quote::format_ident;
use syn::Ident;

use crate::codegen::CodeGenerationContext;



#[derive(Clone, Debug)]
pub struct PreparedFileEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) file_name: String,

    pub(crate) file_path_relative_to_tree_root: String,

    pub(crate) file_data: FileDataConfiguration,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,
}


pub(crate) fn prepare_file_entry(
    context: &mut CodeGenerationContext,
    parent_relative_path: &Path,
    file: &FileEntry,
) -> PreparedFileEntry {
    let friendly_snake_case_field_name = file
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_");

    let preferred_field_name_on_parent = format_ident!("{}", friendly_snake_case_field_name);


    let friendly_upper_camel_case_file_name = file
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join("")
        .to_upper_camel_case();

    let file_struct_name = context
        .struct_name_collision_avoider
        .collision_free_name(&friendly_upper_camel_case_file_name);

    let file_struct_ident = format_ident!("{}", file_struct_name);



    let file_relative_path = parent_relative_path.join(&file.name);
    let file_relative_path_string = file_relative_path
        .to_slash()
        .expect("invalid relative file path: not UTF-8!")
        .to_string();




    PreparedFileEntry {
        entry_id: file.id.to_owned(),
        file_name: file.name.clone(),
        file_path_relative_to_tree_root: file_relative_path_string,
        file_data: file
            .data
            .as_ref()
            .unwrap_or(&FileDataConfiguration::Empty)
            .to_owned(),
        struct_type_ident: file_struct_ident,
        preferred_parent_field_ident: preferred_field_name_on_parent,
    }
}
