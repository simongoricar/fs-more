use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::{PreparedSymlinkEntry, SymlinkEntryError};
use crate::codegen::{AnyPreparedEntry, CodeGenerationContext};



fn construct_post_initializer_code_for_symlink_entry(
    prepared_entry: &PreparedSymlinkEntry,
    symlink_destination_entry: &AnyPreparedEntry,
    symlink_path_variable_ident: &Ident,
    symlink_destination_path_variable_ident: &Ident,
    tree_root_absolute_path_parameter_ident: &Ident,
) -> Result<TokenStream, SymlinkEntryError> {
    match symlink_destination_entry {
        AnyPreparedEntry::Directory { .. } => Ok({
            quote! {
                self.#symlink_path_variable_ident.assert_not_exists();

                let absolute_destination_path =
                    #tree_root_absolute_path_parameter_ident.join(&self.#symlink_destination_path_variable_ident);

                initialize_symbolic_link(
                    &self.#symlink_path_variable_ident,
                    &absolute_destination_path,
                    SymlinkDestinationType::Directory,
                );

                self.#symlink_path_variable_ident.assert_is_valid_symlink_to_directory_and_destination_matches(
                    &absolute_destination_path
                );
            }
        }),
        AnyPreparedEntry::File { .. } => Ok({
            quote! {
                self.#symlink_path_variable_ident.assert_not_exists();

                let absolute_destination_path =
                    #tree_root_absolute_path_parameter_ident.join(&self.#symlink_destination_path_variable_ident);

                initialize_symbolic_link(
                    &self.#symlink_path_variable_ident,
                    &absolute_destination_path,
                    SymlinkDestinationType::File,
                );

                self.#symlink_path_variable_ident.assert_is_valid_symlink_to_file_and_destination_matches(
                    &absolute_destination_path
                );
            }
        }),
        AnyPreparedEntry::Symlink { .. } | AnyPreparedEntry::BrokenSymlink { .. } => {
            Err(SymlinkEntryError::ChainingSymlinksNotSupported {
                from: PathBuf::from(&prepared_entry.symlink_path_relative_to_tree_root),
                to: match symlink_destination_entry {
                    AnyPreparedEntry::Symlink { entry, .. } => {
                        PathBuf::from(&entry.symlink_path_relative_to_tree_root)
                    }
                    _ => unreachable!(),
                },
            })
        }
    }
}



#[derive(Debug, Clone)]
pub(crate) struct GeneratedSymlinkEntry {
    pub(crate) symlink_name: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) generated_code: TokenStream,
}


pub(crate) fn generate_code_for_symlink_entry_in_tree(
    context: &CodeGenerationContext,
    tree_root_struct_ident: &Ident,
    prepared_entry: PreparedSymlinkEntry,
) -> Result<GeneratedSymlinkEntry, SymlinkEntryError> {
    let symlink_path_variable_ident = format_ident!("symlink_path");
    let symlink_destination_path_variable_ident = format_ident!("symlink_destination_path");

    let tree_root_absolute_path_parameter_ident = format_ident!("tree_root_absolute_path");


    let Some(destination_entry) = context
        .prepared_entry_registry
        .entry_by_id(&prepared_entry.symlink_destination_entry_id)
    else {
        return Err(SymlinkEntryError::UnrecognizedDestinationId {
            id: prepared_entry.symlink_destination_entry_id.clone(),
        });
    };

    let symlink_destination_path_relative_to_tree_root =
        destination_entry.path_relative_to_harness_root();


    let documentation_for_symlink_entry = format!(
        "This is a symbolic link residing at `{}` and pointing to `{}`\n\
        (both paths are relative to the root of the test harness).\
        \n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        prepared_entry.symlink_path_relative_to_tree_root,
        symlink_destination_path_relative_to_tree_root,
        tree_root_struct_ident
    );


    let generated_symlink_post_initialization_code =
        construct_post_initializer_code_for_symlink_entry(
            &prepared_entry,
            destination_entry,
            &symlink_path_variable_ident,
            &symlink_destination_path_variable_ident,
            &tree_root_absolute_path_parameter_ident,
        )?;


    let symlink_name = &prepared_entry.symlink_name;

    let symlink_entry_struct_name_ident = &prepared_entry.struct_type_ident;
    let symlink_path_relative_to_tree_root = &prepared_entry.symlink_path_relative_to_tree_root;


    let generated_code_for_symlink_entry = quote! {
        #[doc = #documentation_for_symlink_entry]
        pub struct #symlink_entry_struct_name_ident {
            #symlink_path_variable_ident: PathBuf,

            /// Symlink destination path, relative to the tree harness root.
            #symlink_destination_path_variable_ident: PathBuf,
        }

        impl #symlink_entry_struct_name_ident {
            #[track_caller]
            fn initialize(parent_directory_path: &Path) -> Self {
                let #symlink_path_variable_ident = parent_directory_path.join(#symlink_name);
                let #symlink_destination_path_variable_ident = #symlink_destination_path_relative_to_tree_root.into();

                #symlink_path_variable_ident.assert_not_exists();

                Self {
                    #symlink_path_variable_ident,
                    #symlink_destination_path_variable_ident
                }
            }

            #[track_caller]
            fn post_initialize(&mut self, #tree_root_absolute_path_parameter_ident: &Path) {
                #generated_symlink_post_initialization_code
            }
        }


        impl AsPath for #symlink_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.#symlink_path_variable_ident
            }
        }

        impl AsRelativePath for #symlink_entry_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#symlink_path_relative_to_tree_root)
            }
        }
    };


    Ok(GeneratedSymlinkEntry {
        symlink_name: prepared_entry.symlink_name,
        struct_type_ident: prepared_entry.struct_type_ident,
        documentation_for_parent_field: documentation_for_symlink_entry,
        generated_code: generated_code_for_symlink_entry,
    })
}
