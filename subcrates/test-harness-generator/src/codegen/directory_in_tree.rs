use std::path::Path;

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use thiserror::Error;

use super::{
    file_in_tree::{generate_code_for_file_entry_in_tree, prepare_file_entry},
    symlink_in_tree::SymlinkEntryError,
    AnyPreparedEntry,
    CodeGenerationContext,
    TreeRegistryError,
};
use crate::{
    codegen::symlink_in_tree::{generate_code_for_symlink_entry_in_tree, prepare_symlink_entry},
    name_collision::NameCollisionAvoider,
    schema::{DirectoryEntry, FileSystemHarnessEntry},
};


#[derive(Debug, Error)]
pub enum DirectoryEntryError {
    #[error("symlink entry failed to prepare or generate")]
    SymlinkSubEntryError(
        #[from]
        #[source]
        SymlinkEntryError,
    ),

    #[error("failed to register tree entry into registry")]
    TreeEntryRegistryError(
        #[from]
        #[source]
        TreeRegistryError,
    ),
}


#[derive(Clone, Debug)]
pub struct PreparedDirectoryEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) directory_name: String,

    pub(crate) directory_path_relative_to_tree_root: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) documentation_for_struct: String,

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
    tree_root_struct_ident: &Ident,
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
                let prepared_file_entry = prepare_file_entry(
                    context,
                    tree_root_struct_ident,
                    &directory_relative_path,
                    file_entry,
                );

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_file_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::File {
                    entry: prepared_file_entry,
                    actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Directory(directory_entry) => {
                let prepared_directory_entry = prepare_directory_entry(
                    context,
                    tree_root_struct_ident,
                    &directory_relative_path,
                    directory_entry,
                )?;

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_directory_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Directory {
                    entry: prepared_directory_entry,
                    actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
            FileSystemHarnessEntry::Symlink(symlink_entry) => {
                let prepared_symlink_entry = prepare_symlink_entry(
                    context,
                    tree_root_struct_ident,
                    &directory_relative_path,
                    symlink_entry,
                )?;

                let actual_field_name_ident_on_parent = struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_symlink_entry.preferred_parent_field_ident);


                let prepared_entry_any = AnyPreparedEntry::Symlink {
                    entry: prepared_symlink_entry,
                    actual_field_name_ident_on_parent,
                };

                context
                    .prepared_entry_registry
                    .add_prepared_entry(prepared_entry_any.clone())?;

                prepared_entries.push(prepared_entry_any);
            }
        }
    }



    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum StructFieldType {
        File,
        Directory,
        Symlink,
    }

    struct StructFieldAnnotation {
        formatted_field: String,
        field_type: StructFieldType,
    }

    let mut formatted_available_fields_on_struct =
        Vec::with_capacity(unparsed_directory_entries.len());
    for entry in &prepared_entries {
        let (field_type, formatted_field) = match entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.directory_name,
                    actual_field_name_ident_on_parent,
                    entry.struct_type_ident
                );

                (StructFieldType::Directory, formatted_field)
            }
            AnyPreparedEntry::File {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.file_name, actual_field_name_ident_on_parent, entry.struct_type_ident
                );

                (StructFieldType::File, formatted_field)
            }
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.symlink_name, actual_field_name_ident_on_parent, entry.struct_type_ident
                );

                (StructFieldType::Symlink, formatted_field)
            }
        };

        formatted_available_fields_on_struct.push(StructFieldAnnotation {
            field_type,
            formatted_field,
        });
    }



    let documentation_on_available_fields = {
        let available_directory_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::Directory)
            .map(|entry| entry.formatted_field.as_str())
            .collect_vec();

        let available_file_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::File)
            .map(|entry| entry.formatted_field.as_str())
            .collect_vec();

        let available_symlink_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::Symlink)
            .map(|entry| entry.formatted_field.as_str())
            .collect_vec();


        let mut documentation_segments = Vec::with_capacity(3);

        if !available_directory_fields.is_empty() {
            documentation_segments.push(format!(
                "It contains the following sub-directories:\n{}",
                available_directory_fields.join("\n")
            ));
        }

        if !available_file_fields.is_empty() {
            documentation_segments.push(format!(
                "It contains the following files:\n{}",
                available_file_fields.join("\n")
            ));
        }

        if !available_symlink_fields.is_empty() {
            documentation_segments.push(format!(
                "It contains the following symlinks:\n{}",
                available_symlink_fields.join("\n")
            ));
        }

        format!("\n{}\n", documentation_segments.join("\n"))
    };

    let documentation_for_entry = format!(
        "This is a sub-directory residing at `{}` (relative to the root of the test harness).\
        \n\n{}\n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        parent_relative_path.join(&directory.name).to_slash_lossy(),
        documentation_on_available_fields,
        tree_root_struct_ident
    );


    Ok(PreparedDirectoryEntry {
        entry_id: directory.id.to_owned(),
        directory_name: directory.name.to_owned(),
        directory_path_relative_to_tree_root: directory_relative_path_string,
        struct_type_ident: directory_struct_name_ident,
        preferred_parent_field_ident: preferred_field_name_on_parent,
        documentation_for_struct: documentation_for_entry.clone(),
        documentation_for_parent_field: documentation_for_entry,
        entries: prepared_entries,
    })
}


pub(crate) struct GeneratedHarnessDirectoryEntry {
    // TODO remove
    /*
    pub(crate) struct_type_name: String,

    pub(crate) preferred_parent_field_name: String,

    pub(crate) parent_field_documentation: String, */
    /// Includes the concatenated code from all descendant entries.
    pub(crate) generated_code: TokenStream,
}



fn construct_struct_fields_for_entries(entries: &[AnyPreparedEntry]) -> Vec<TokenStream> {
    let mut field_specifiers = Vec::with_capacity(entries.len());

    for entry in entries {
        let (field_name_ident, field_type_ident, field_documentation) = match entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_ident_on_parent,
            } => (
                actual_field_name_ident_on_parent,
                &entry.struct_type_ident,
                &entry.documentation_for_parent_field,
            ),
            AnyPreparedEntry::File {
                entry,
                actual_field_name_ident_on_parent,
            } => (
                actual_field_name_ident_on_parent,
                &entry.struct_type_ident,
                &entry.documentation_for_parent_field,
            ),
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_ident_on_parent,
            } => (
                actual_field_name_ident_on_parent,
                &entry.struct_type_ident,
                &entry.documentation_for_parent_field,
            ),
        };

        field_specifiers.push(quote! {
            #[doc = #field_documentation]
            pub #field_name_ident: #field_type_ident
        });
    }

    field_specifiers
}


fn construct_initializers_for_entries(
    entries: &[AnyPreparedEntry],
    directory_path_variable_ident: &Ident,
) -> Vec<TokenStream> {
    let mut field_initializers = Vec::with_capacity(entries.len());

    for entry in entries {
        let initializer = match entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = &entry.struct_type_ident;
                let directory_name_string_value = &entry.directory_name;

                quote! {
                    let #actual_field_name_ident_on_parent = <#field_type_ident>::initialize(
                        &#directory_path_variable_ident,
                        #directory_name_string_value
                    );
                }
            }
            AnyPreparedEntry::File {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = &entry.struct_type_ident;
                let file_name_string_value = &entry.file_name;

                quote! {
                    let #actual_field_name_ident_on_parent = <#field_type_ident>::initialize(
                        &#directory_path_variable_ident,
                        #file_name_string_value
                    );
                }
            }
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = &entry.struct_type_ident;
                let symlink_name_string_value = &entry.symlink_name;

                quote! {
                    let #actual_field_name_ident_on_parent = <#field_type_ident>::initialize(
                        &#directory_path_variable_ident,
                        #symlink_name_string_value
                    );
                }
            }
        };

        field_initializers.push(initializer);
    }

    field_initializers
}




fn construct_final_struct_initializer(
    entries: &[AnyPreparedEntry],
    directory_path_variable_ident: &Ident,
) -> TokenStream {
    let mut field_name_idents = Vec::with_capacity(entries.len());
    for entry in entries {
        let field_ident = match entry {
            AnyPreparedEntry::Directory {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
            AnyPreparedEntry::File {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
            AnyPreparedEntry::Symlink {
                actual_field_name_ident_on_parent,
                ..
            } => actual_field_name_ident_on_parent,
        };

        field_name_idents.push(field_ident);
    }


    quote! {
        Self {
            #directory_path_variable_ident,
            #(#field_name_idents),*
        }
    }
}

fn construct_post_initializers_for_entries(entries: &[AnyPreparedEntry]) -> Vec<TokenStream> {
    let num_symlink_entries = entries
        .iter()
        .filter(|entry| matches!(entry, AnyPreparedEntry::Symlink { .. }))
        .count();

    let mut field_post_initializers = Vec::with_capacity(num_symlink_entries);

    for entry in entries {
        let AnyPreparedEntry::Symlink {
            actual_field_name_ident_on_parent,
            ..
        } = entry
        else {
            continue;
        };

        field_post_initializers.push(quote! {
            self.#actual_field_name_ident_on_parent.post_initialize();
        });
    }

    field_post_initializers
}

fn construct_post_initialize_function_if_needed(
    prepared_directory: &PreparedDirectoryEntry,
) -> Option<TokenStream> {
    if !prepared_directory.requires_post_initialization_call() {
        return None;
    }


    let post_initializer_calls =
        construct_post_initializers_for_entries(&prepared_directory.entries);

    Some(quote! {
        #[track_caller]
        fn post_initialize(&mut self) {
            #(#post_initializer_calls)*
        }
    })
}



pub(crate) fn generate_code_for_directory_entry_in_tree(
    context: &CodeGenerationContext,
    tree_root_struct_ident: &Ident,
    prepared_directory: PreparedDirectoryEntry,
) -> Result<GeneratedHarnessDirectoryEntry, DirectoryEntryError> {
    let directory_entry_struct_name_ident = &prepared_directory.struct_type_ident;
    let directory_entry_documentation = &prepared_directory.documentation_for_struct;

    let directory_struct_fields = construct_struct_fields_for_entries(&prepared_directory.entries);

    let directory_path_variable_ident = format_ident!("directory_path");
    let directory_entry_initializers = construct_initializers_for_entries(
        &prepared_directory.entries,
        &directory_path_variable_ident,
    );

    let final_struct_initializer = construct_final_struct_initializer(
        &prepared_directory.entries,
        &directory_path_variable_ident,
    );

    let directory_path_relative_to_tree_root =
        &prepared_directory.directory_path_relative_to_tree_root;



    let potential_post_initialize_function =
        construct_post_initialize_function_if_needed(&prepared_directory);


    let mut token_stream_to_prepend = TokenStream::new();

    for prepared_sub_entry in prepared_directory.entries {
        match prepared_sub_entry {
            AnyPreparedEntry::Directory { entry, .. } => {
                let generated_directory_entry = generate_code_for_directory_entry_in_tree(
                    context,
                    tree_root_struct_ident,
                    entry,
                )?;

                let generated_code = &generated_directory_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };
            }
            AnyPreparedEntry::File { entry, .. } => {
                let generated_directory_entry =
                    generate_code_for_file_entry_in_tree(context, tree_root_struct_ident, entry);

                let generated_code = &generated_directory_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };
            }
            AnyPreparedEntry::Symlink { entry, .. } => {
                let generated_directory_entry =
                    generate_code_for_symlink_entry_in_tree(context, entry)?;

                let generated_code = &generated_directory_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };
            }
        }
    }



    let generated_code_for_directory_entry = quote! {
        #token_stream_to_prepend

        #[doc = #directory_entry_documentation]
        pub struct #directory_entry_struct_name_ident {
            #directory_path_variable_ident: PathBuf,
            #(#directory_struct_fields),*
        }

        impl #directory_entry_struct_name_ident {
            #[track_caller]
            fn initialize<S>(parent_directory_path: &Path, directory_name: S) -> Self
            where
                S: Into<String>
            {
                let #directory_path_variable_ident = parent_directory_path.join(directory_name.into());

                #directory_path_variable_ident.assert_not_exists();
                fs::create_dir(&directory_path)
                    .expect("failed to create directory");
                #directory_path_variable_ident.assert_is_directory_and_empty();


                #(#directory_entry_initializers)*


                #final_struct_initializer
            }

            #potential_post_initialize_function
        }

        impl AsPath for #directory_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.#directory_path_variable_ident
            }
        }

        impl AsRelativePath for #directory_entry_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#directory_path_relative_to_tree_root)
            }
        }

        impl FileSystemHarnessDirectory for #directory_entry_struct_name_ident {}
    };


    Ok(GeneratedHarnessDirectoryEntry {
        generated_code: generated_code_for_directory_entry,
    })
}
