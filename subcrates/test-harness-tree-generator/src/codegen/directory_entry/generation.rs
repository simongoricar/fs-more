use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::{DirectoryEntryError, PreparedDirectoryEntry};
use crate::codegen::{
    broken_symlink_entry::generate_code_for_broken_symlink_entry_in_tree,
    file_entry::generate_code_for_file_entry_in_tree,
    symlink_entry::generate_code_for_symlink_entry_in_tree,
    AnyGeneratedEntry,
    AnyPreparedEntry,
    CodeGenerationContext,
};



#[derive(Debug, Clone)]
pub(crate) struct GeneratedDirectoryEntry {
    pub(crate) directory_name: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    /// Includes the concatenated code from all descendant entries.
    pub(crate) generated_code: TokenStream,
}



fn construct_struct_fields_for_entries(
    generated_entries: &[AnyGeneratedEntry],
) -> Vec<TokenStream> {
    let mut field_specifiers = Vec::with_capacity(generated_entries.len());

    for generated_entry in generated_entries {
        let field_name_ident = generated_entry.actual_field_name_ident_on_parent();
        let field_type_ident = generated_entry.struct_type_ident();
        let field_documentation = generated_entry.documentation_for_parent_field();

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
        let field_name_on_parent_ident = entry.actual_field_name_on_parent_ident();
        let struct_type_ident = entry.struct_type_ident();

        field_initializers.push(quote! {
            let #field_name_on_parent_ident = <#struct_type_ident>::initialize(
                &#directory_path_variable_ident
            );
        });
    }

    field_initializers
}


fn construct_final_struct_initializer(
    entries: &[AnyPreparedEntry],
    directory_path_variable_ident: &Ident,
) -> TokenStream {
    let mut field_name_idents = Vec::with_capacity(entries.len());
    for entry in entries {
        let field_name_on_parent_ident = entry.actual_field_name_on_parent_ident();

        field_name_idents.push(field_name_on_parent_ident);
    }


    quote! {
        Self {
            #directory_path_variable_ident,
            #(#field_name_idents),*
        }
    }
}


fn construct_post_initializers_for_entries(
    entries: &[AnyPreparedEntry],
    tree_root_absolute_path_parameter_ident: &Ident,
) -> Vec<TokenStream> {
    let num_symlink_entries = entries
        .iter()
        .filter(|entry| matches!(entry, AnyPreparedEntry::Symlink { .. }))
        .count();

    let mut field_post_initializers = Vec::with_capacity(num_symlink_entries);

    for entry in entries {
        match entry {
            AnyPreparedEntry::Directory {
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                entry,
            } => {
                if !entry.requires_post_initialization_call() {
                    continue;
                }

                field_post_initializers.push(quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize(
                        #tree_root_absolute_path_parameter_ident
                    );
                });
            }
            AnyPreparedEntry::Symlink {
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                ..
            } => {
                field_post_initializers.push(quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize(
                        #tree_root_absolute_path_parameter_ident
                    );
                });
            }
            AnyPreparedEntry::BrokenSymlink {
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
                ..
            } => {
                field_post_initializers.push(quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize(
                        #tree_root_absolute_path_parameter_ident
                    );
                });
            }
            _ => {}
        }
    }

    field_post_initializers
}


fn construct_post_initialize_function_if_needed(
    prepared_directory: &PreparedDirectoryEntry,
    tree_root_absolute_path_parameter_ident: &Ident,
) -> Option<TokenStream> {
    if !prepared_directory.requires_post_initialization_call() {
        return None;
    }


    let post_initializer_calls = construct_post_initializers_for_entries(
        &prepared_directory.entries,
        tree_root_absolute_path_parameter_ident,
    );

    Some(quote! {
        #[track_caller]
        fn post_initialize(&mut self, #tree_root_absolute_path_parameter_ident: &Path) {
            #(#post_initializer_calls)*
        }
    })
}


fn construct_documentation_for_directory_entry(
    directory_path_relative_to_tree_root: &str,
    tree_root_struct_ident: &Ident,
    generated_entries: &[AnyGeneratedEntry],
) -> String {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum StructFieldType {
        File,
        Directory,
        Symlink,
        BrokenSymlink,
    }

    struct StructFieldAnnotation {
        field_formatted: String,
        field_type: StructFieldType,
    }

    let mut formatted_available_fields_on_struct = Vec::with_capacity(generated_entries.len());
    for entry in generated_entries {
        let (field_type, field_formatted) = match entry {
            AnyGeneratedEntry::Directory {
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
            AnyGeneratedEntry::File {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.file_name, actual_field_name_ident_on_parent, entry.struct_type_ident
                );

                (StructFieldType::File, formatted_field)
            }
            AnyGeneratedEntry::Symlink {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.symlink_name, actual_field_name_ident_on_parent, entry.struct_type_ident
                );

                (StructFieldType::Symlink, formatted_field)
            }
            AnyGeneratedEntry::BrokenSymlink {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let formatted_field = format!(
                    "- `{}` (field `{}`; see [`{}`])",
                    entry.symlink_name, actual_field_name_ident_on_parent, entry.struct_type_ident
                );

                (StructFieldType::BrokenSymlink, formatted_field)
            }
        };

        formatted_available_fields_on_struct.push(StructFieldAnnotation {
            field_type,
            field_formatted,
        });
    }

    let documentation_on_available_fields = {
        let available_directory_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::Directory)
            .map(|entry| entry.field_formatted.as_str())
            .collect_vec();

        let available_file_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::File)
            .map(|entry| entry.field_formatted.as_str())
            .collect_vec();

        let available_symlink_fields = formatted_available_fields_on_struct
            .iter()
            .filter(|entry| entry.field_type == StructFieldType::Symlink)
            .map(|entry| entry.field_formatted.as_str())
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

        format!("\n{}\n", documentation_segments.join("\n\n"))
    };

    format!(
        "This is a sub-directory residing at `{}` (relative to the root of the test harness).\
        \n\n{}\n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        directory_path_relative_to_tree_root,
        documentation_on_available_fields,
        tree_root_struct_ident
    )
}



pub(crate) fn generate_code_for_directory_entry_in_tree(
    context: &CodeGenerationContext,
    tree_root_struct_ident: &Ident,
    prepared_directory: PreparedDirectoryEntry,
) -> Result<GeneratedDirectoryEntry, DirectoryEntryError> {
    let directory_entry_struct_name_ident = &prepared_directory.struct_type_ident;

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


    let tree_root_absolute_path_parameter_ident = format_ident!("tree_root_absolute_path");


    let potential_post_initialize_function = construct_post_initialize_function_if_needed(
        &prepared_directory,
        &tree_root_absolute_path_parameter_ident,
    );


    let mut token_stream_to_prepend = TokenStream::new();
    let mut generated_entries = Vec::with_capacity(prepared_directory.entries.len());

    for prepared_sub_entry in prepared_directory.entries {
        match prepared_sub_entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
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

                generated_entries.push(AnyGeneratedEntry::Directory {
                    entry: generated_directory_entry,
                    actual_field_name_ident_on_parent,
                });
            }
            AnyPreparedEntry::File {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let generated_file_entry =
                    generate_code_for_file_entry_in_tree(tree_root_struct_ident, entry);

                let generated_code = &generated_file_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };

                generated_entries.push(AnyGeneratedEntry::File {
                    entry: generated_file_entry,
                    actual_field_name_ident_on_parent,
                });
            }
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let generated_symlink_entry = generate_code_for_symlink_entry_in_tree(
                    context,
                    tree_root_struct_ident,
                    entry,
                )?;

                let generated_code = &generated_symlink_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };

                generated_entries.push(AnyGeneratedEntry::Symlink {
                    entry: generated_symlink_entry,
                    actual_field_name_ident_on_parent,
                });
            }
            AnyPreparedEntry::BrokenSymlink {
                entry,
                actual_field_name_on_parent_ident: actual_field_name_ident_on_parent,
            } => {
                let generated_broken_symlink_entry =
                    generate_code_for_broken_symlink_entry_in_tree(tree_root_struct_ident, entry)?;

                let generated_code = &generated_broken_symlink_entry.generated_code;


                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };

                generated_entries.push(AnyGeneratedEntry::BrokenSymlink {
                    entry: generated_broken_symlink_entry,
                    actual_field_name_ident_on_parent,
                });
            }
        }
    }


    let directory_struct_fields = construct_struct_fields_for_entries(&generated_entries);

    let directory_entry_documentation = construct_documentation_for_directory_entry(
        &prepared_directory.directory_path_relative_to_tree_root,
        tree_root_struct_ident,
        &generated_entries,
    );

    let directory_name = &prepared_directory.directory_name;


    let generated_code_for_directory_entry = quote! {
        #token_stream_to_prepend

        #[doc = #directory_entry_documentation]
        pub struct #directory_entry_struct_name_ident {
            #directory_path_variable_ident: PathBuf,
            #(#directory_struct_fields),*
        }

        impl #directory_entry_struct_name_ident {
            #[track_caller]
            fn initialize(parent_directory_path: &Path) -> Self
            {
                let #directory_path_variable_ident = parent_directory_path.join(#directory_name);

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


    Ok(GeneratedDirectoryEntry {
        directory_name: prepared_directory.directory_name,
        struct_type_ident: prepared_directory.struct_type_ident,
        documentation_for_parent_field: directory_entry_documentation,
        generated_code: generated_code_for_directory_entry,
    })
}
