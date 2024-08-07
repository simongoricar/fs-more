use std::path::Path;

use fs_more_test_harness_tree_schema::schema::{FileSystemHarnessEntry, FileSystemHarnessSchema};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use super::SchemaCodeGenerationError;
use crate::{
    codegen::{
        broken_symlink_entry::prepare_broken_symlink_entry,
        directory_entry::prepare_directory_entry,
        file_entry::prepare_file_entry,
        symlink_entry::prepare_symlink_entry,
        AnyPreparedEntry,
        CodeGenerationContext,
    },
    name_collision::NameCollisionAvoider,
};



pub(super) fn prepare_tree_entries(
    schema: &FileSystemHarnessSchema,
    global_context: &mut CodeGenerationContext,
    tree_root_relative_path: &Path,
) -> Result<Vec<AnyPreparedEntry>, SchemaCodeGenerationError> {
    let mut local_struct_field_name_collision_avoider = NameCollisionAvoider::new_empty();

    let mut prepared_entries = Vec::with_capacity(schema.structure.entries.len());
    for entry in &schema.structure.entries {
        match entry {
            FileSystemHarnessEntry::File(file_entry) => {
                let prepared_file_entry =
                    prepare_file_entry(global_context, tree_root_relative_path, file_entry);

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_file_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::File {
                    entry: prepared_file_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                global_context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())
                    .map_err(|error| SchemaCodeGenerationError::TreeRegistryError {
                        error,
                        entry_relative_path: tree_root_relative_path.join(&file_entry.name),
                    })?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Directory(directory_entry) => {
                let prepared_directory_entry = prepare_directory_entry(
                    global_context,
                    tree_root_relative_path,
                    directory_entry,
                )
                .map_err(|error| {
                    SchemaCodeGenerationError::DirectoryEntryError {
                        error,
                        directory_relative_path: tree_root_relative_path
                            .join(&directory_entry.name),
                    }
                })?;

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_directory_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Directory {
                    entry: prepared_directory_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                global_context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())
                    .map_err(|error| SchemaCodeGenerationError::TreeRegistryError {
                        error,
                        entry_relative_path: tree_root_relative_path.join(&directory_entry.name),
                    })?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Symlink(symlink_entry) => {
                let prepared_symlink_entry =
                    prepare_symlink_entry(global_context, tree_root_relative_path, symlink_entry)
                        .map_err(|error| SchemaCodeGenerationError::SymlinkEntryError {
                        error,
                        symlink_relative_path: tree_root_relative_path.join(&symlink_entry.name),
                    })?;

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_symlink_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Symlink {
                    entry: prepared_symlink_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                global_context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())
                    .map_err(|error| SchemaCodeGenerationError::TreeRegistryError {
                        error,
                        entry_relative_path: tree_root_relative_path.join(&symlink_entry.name),
                    })?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::BrokenSymlink(broken_symlink_entry) => {
                let prepared_broken_symlink_entry = prepare_broken_symlink_entry(
                    global_context,
                    tree_root_relative_path,
                    broken_symlink_entry,
                )
                .map_err(|error| {
                    SchemaCodeGenerationError::BrokenSymlinkEntryError {
                        error,
                        broken_symlink_relative_path: tree_root_relative_path
                            .join(&broken_symlink_entry.name),
                    }
                })?;

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(
                        &prepared_broken_symlink_entry.preferred_parent_field_ident,
                    );


                let prepared_entry_any = AnyPreparedEntry::BrokenSymlink {
                    entry: prepared_broken_symlink_entry,
                    actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                };

                global_context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())
                    .map_err(|error| SchemaCodeGenerationError::TreeRegistryError {
                        error,
                        entry_relative_path: tree_root_relative_path
                            .join(&broken_symlink_entry.name),
                    })?;

                prepared_entries.push(prepared_entry_any);
            }
        }
    }

    Ok(prepared_entries)
}


pub(super) fn construct_field_initializer_code(
    prepared_entries: &[AnyPreparedEntry],
    temporary_directory_path_variable_ident: &Ident,
) -> TokenStream {
    let mut individual_initializers = Vec::with_capacity(prepared_entries.len());

    for entry in prepared_entries {
        let field_name_on_parent_ident = entry.actual_field_name_on_parent_ident();
        let struct_type_ident = entry.struct_type_ident();

        individual_initializers.push(quote! {
            let #field_name_on_parent_ident = <#struct_type_ident>::initialize(
                #temporary_directory_path_variable_ident
            );
        });
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(individual_initializers);
    final_token_stream
}



pub(super) fn construct_field_post_initializer_code(
    prepared_entries: &[AnyPreparedEntry],
    temporary_directory_struct_field_ident: &Ident,
) -> Option<TokenStream> {
    let mut individual_initializers = Vec::with_capacity(prepared_entries.len());

    for entry in prepared_entries {
        let initializer = match entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                if !entry.requires_post_initialization_call() {
                    continue;
                }

                quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize(
                        self.#temporary_directory_struct_field_ident.path()
                    );
                }
            }
            AnyPreparedEntry::Symlink {
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                ..
            } => {
                quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize(
                        self.#temporary_directory_struct_field_ident.path()
                    );
                }
            }
            AnyPreparedEntry::BrokenSymlink {
                actual_field_name_on_parent_ident,
                ..
            } => {
                quote! {
                    self.#actual_field_name_on_parent_ident.post_initialize(
                        self.#temporary_directory_struct_field_ident.path()
                    );
                }
            }
            AnyPreparedEntry::File { .. } => continue,
        };

        individual_initializers.push(initializer);
    }


    if individual_initializers.is_empty() {
        return None;
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(individual_initializers);
    Some(final_token_stream)
}
