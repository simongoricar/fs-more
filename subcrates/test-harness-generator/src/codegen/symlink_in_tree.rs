use std::path::{Path, PathBuf};

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use thiserror::Error;

use super::{AnyPreparedEntry, CodeGenerationContext};
use crate::schema::SymlinkEntry;


#[derive(Debug, Error)]
pub enum SymlinkEntryError {
    #[error("unrecognized symlink destination entry ID: {id}")]
    UnrecognizedDestinationId { id: String },

    #[error(
        "chaining symlinks is currently not supported (tried to chain from {} to {})",
        .from.display(),
        .to.display()
    )]
    ChainingSymlinksNotSupported { from: PathBuf, to: PathBuf },
}


#[derive(Clone, Debug)]
pub struct PreparedSymlinkEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) symlink_name: String,

    pub(crate) symlink_path_relative_to_tree_root: String,

    pub(crate) symlink_destination_entry_id: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) documentation_for_struct: String,
}


pub(crate) fn prepare_symlink_entry(
    context: &mut CodeGenerationContext,
    tree_root_struct_ident: &Ident,
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


    let documentation_for_symlink_entry = format!(
        "This is a symbolic link residing at `{}` (relative to the root of the test harness).\
        \n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        symlink_relative_path_string, tree_root_struct_ident
    );



    Ok(PreparedSymlinkEntry {
        entry_id: symlink.id.clone(),
        symlink_name: symlink.name.clone(),
        symlink_path_relative_to_tree_root: symlink_relative_path_string,
        symlink_destination_entry_id: symlink.destination_entry_id.clone(),
        struct_type_ident: symlink_struct_ident,
        preferred_parent_field_ident,
        documentation_for_struct: documentation_for_symlink_entry.clone(),
        documentation_for_parent_field: documentation_for_symlink_entry,
    })
}



fn construct_post_initializer_code_for_symlink_entry(
    prepared_entry: &PreparedSymlinkEntry,
    symlink_destination_entry: &AnyPreparedEntry,
    symlink_path_variable_ident: &Ident,
    symlink_destination_path_variable_ident: &Ident,
) -> Result<TokenStream, SymlinkEntryError> {
    match symlink_destination_entry {
        AnyPreparedEntry::Directory { .. } => Ok({
            quote! {
                self.#symlink_path_variable_ident.assert_not_exists();

                initialize_symbolic_link(
                    &self.#symlink_path_variable_ident,
                    &self.#symlink_destination_path_variable_ident,
                    SymlinkDestinationType::Directory,
                );

                self.#symlink_path_variable_ident.assert_is_symlink_to_directory();
            }
        }),
        AnyPreparedEntry::File { .. } => Ok({
            quote! {
                self.#symlink_path_variable_ident.assert_not_exists();

                initialize_symbolic_link(
                    &self.#symlink_path_variable_ident,
                    &self.#symlink_destination_path_variable_ident,
                    SymlinkDestinationType::File,
                );

                self.#symlink_path_variable_ident.assert_is_symlink_to_directory();
            }
        }),
        AnyPreparedEntry::Symlink { .. } => Err(SymlinkEntryError::ChainingSymlinksNotSupported {
            from: PathBuf::from(&prepared_entry.symlink_path_relative_to_tree_root),
            to: match symlink_destination_entry {
                AnyPreparedEntry::Symlink { entry, .. } => {
                    PathBuf::from(&entry.symlink_path_relative_to_tree_root)
                }
                _ => unreachable!(),
            },
        }),
    }
}



pub(crate) struct GeneratedSymlinkEntry {
    // TODO remove
    /*
        pub(crate) struct_type_name: String,

        pub(crate) preferred_parent_field_name: String,

        pub(crate) parent_field_documentation: String,
    */
    pub(crate) generated_code: TokenStream,
}


pub(crate) fn generate_code_for_symlink_entry_in_tree(
    context: &CodeGenerationContext,
    prepared_entry: PreparedSymlinkEntry,
) -> Result<GeneratedSymlinkEntry, SymlinkEntryError> {
    let symlink_path_variable_ident = format_ident!("symlink_path");
    let symlink_destination_path_variable_ident = format_ident!("symlink_destination_path");


    let Some(destination_entry) = context
        .prepared_entry_registry
        .entry_by_id(&prepared_entry.symlink_destination_entry_id)
    else {
        return Err(SymlinkEntryError::UnrecognizedDestinationId {
            id: prepared_entry.symlink_destination_entry_id.clone(),
        });
    };

    let symlink_destination_path_relative_to_tree_root = match destination_entry {
        AnyPreparedEntry::Directory { entry, .. } => &entry.directory_path_relative_to_tree_root,
        AnyPreparedEntry::File { entry, .. } => &entry.file_path_relative_to_tree_root,
        AnyPreparedEntry::Symlink { entry, .. } => &entry.symlink_path_relative_to_tree_root,
    };


    let generated_symlink_post_initialization_code =
        construct_post_initializer_code_for_symlink_entry(
            &prepared_entry,
            destination_entry,
            &symlink_path_variable_ident,
            &symlink_destination_path_variable_ident,
        )?;



    let symlink_entry_struct_name_ident = &prepared_entry.struct_type_ident;
    let symlink_entry_documentation = &prepared_entry.documentation_for_struct;
    let symlink_path_relative_to_tree_root = &prepared_entry.symlink_path_relative_to_tree_root;


    let generated_code_for_symlink_entry = quote! {
        #[doc = #symlink_entry_documentation]
        pub struct #symlink_entry_struct_name_ident {
            #symlink_path_variable_ident: PathBuf,

            // Symbolic link destination path, relative to the root of the tree.
            symlink_destination_path: &'static Path,
        }

        impl #symlink_entry_struct_name_ident {
            #[track_caller]
            fn initialize<S>(
                parent_directory_path: &Path,
                symlink_name: S,
                symlink_destination_path: PathBuf
            ) -> Self
            where
                S: Into<String>
            {
                let #symlink_path_variable_ident = parent_directory_path.join(symlink_name.into());
                let #symlink_destination_path_variable_ident = Path::new(#symlink_destination_path_relative_to_tree_root);

                #symlink_path_variable_ident.assert_not_exists();

                Self {
                    #symlink_path_variable_ident,
                    symlink_destination_path
                }
            }

            #[track_caller]
            fn post_initialize(&mut self) {
                #generated_symlink_post_initialization_code
            }
        }


        impl AsPath for #symlink_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.symlink_path
            }
        }

        impl AsRelativePath for #symlink_entry_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#symlink_path_relative_to_tree_root)
            }
        }
    };


    Ok(GeneratedSymlinkEntry {
        generated_code: generated_code_for_symlink_entry,
    })
}
