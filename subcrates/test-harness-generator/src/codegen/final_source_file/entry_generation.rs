use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use super::SchemaCodeGenerationError;
use crate::codegen::{
    directory_entry::generate_code_for_directory_entry_in_tree,
    file_entry::generate_code_for_file_entry_in_tree,
    symlink_entry::generate_code_for_symlink_entry_in_tree,
    AnyGeneratedEntry,
    AnyPreparedEntry,
    CodeGenerationContext,
};



pub(super) struct TreeEntriesGenerationOutput {
    pub(super) generated_entries: Vec<AnyGeneratedEntry>,

    pub(super) code_to_prepend: TokenStream,

    pub(super) struct_field_specifiers: Vec<TokenStream>,

    pub(super) struct_field_names: Vec<TokenStream>,
}


pub(super) fn generate_code_for_all_tree_sub_entries(
    context: &CodeGenerationContext,
    tree_root_struct_name_ident: &Ident,
    prepared_entries: Vec<AnyPreparedEntry>,
) -> Result<TreeEntriesGenerationOutput, SchemaCodeGenerationError> {
    let mut code_chunks_to_prepend = Vec::with_capacity(prepared_entries.len());

    let mut struct_field_specifiers = Vec::with_capacity(prepared_entries.len());
    let mut struct_field_names = Vec::with_capacity(prepared_entries.len());

    let mut generated_entries = Vec::with_capacity(prepared_entries.len());


    for prepared_entry in prepared_entries {
        let (generated_entry, struct_field_specifier, struct_field_name) = match prepared_entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();
                let directory_path_relative_to_tree_root =
                    entry.directory_path_relative_to_tree_root.clone();


                let generated_directory_entry = generate_code_for_directory_entry_in_tree(
                    context,
                    tree_root_struct_name_ident,
                    entry,
                )
                .map_err(|error| {
                    SchemaCodeGenerationError::DirectoryEntryError {
                        error,
                        directory_relative_path: directory_path_relative_to_tree_root.into(),
                    }
                })?;


                let struct_field_specifier = quote! {
                    #actual_field_name_ident_on_parent: #field_type_ident
                };

                let struct_field_name = quote! {
                    #actual_field_name_ident_on_parent
                };


                (
                    AnyGeneratedEntry::Directory {
                        entry: generated_directory_entry,
                        actual_field_name_ident_on_parent,
                    },
                    struct_field_specifier,
                    struct_field_name,
                )
            }
            AnyPreparedEntry::File {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();

                let generated_directory_entry =
                    generate_code_for_file_entry_in_tree(tree_root_struct_name_ident, entry);


                let struct_field_specifier = quote! {
                    #actual_field_name_ident_on_parent: #field_type_ident
                };

                let struct_field_name = quote! {
                    #actual_field_name_ident_on_parent
                };


                (
                    AnyGeneratedEntry::File {
                        entry: generated_directory_entry,
                        actual_field_name_ident_on_parent,
                    },
                    struct_field_specifier,
                    struct_field_name,
                )
            }
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();
                let symlink_path_relative_to_tree_root =
                    entry.symlink_path_relative_to_tree_root.clone();


                let generated_symlink_entry = generate_code_for_symlink_entry_in_tree(
                    context,
                    tree_root_struct_name_ident,
                    entry,
                )
                .map_err(|error| SchemaCodeGenerationError::SymlinkEntryError {
                    error,
                    symlink_relative_path: symlink_path_relative_to_tree_root.into(),
                })?;


                let struct_field_specifier = quote! {
                    #actual_field_name_ident_on_parent: #field_type_ident
                };

                let struct_field_name = quote! {
                    #actual_field_name_ident_on_parent
                };


                (
                    AnyGeneratedEntry::Symlink {
                        entry: generated_symlink_entry,
                        actual_field_name_ident_on_parent,
                    },
                    struct_field_specifier,
                    struct_field_name,
                )
            }
        };


        code_chunks_to_prepend.push(generated_entry.generated_code().to_owned());
        generated_entries.push(generated_entry);
        struct_field_specifiers.push(struct_field_specifier);
        struct_field_names.push(struct_field_name);
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(code_chunks_to_prepend);

    Ok(TreeEntriesGenerationOutput {
        generated_entries,
        code_to_prepend: final_token_stream,
        struct_field_names,
        struct_field_specifiers,
    })
}
