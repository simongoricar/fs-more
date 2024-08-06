use fs_more_test_harness_tree_schema::schema::SymlinkDestinationType;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::{BrokenSymlinkEntryError, PreparedBrokenSymlinkEntry};


fn construct_post_initializer_code_for_broken_symlink_entry(
    symlink_destination_type: SymlinkDestinationType,
    broken_symlink_path_variable_ident: &Ident,
    broken_symlink_destination_path_variable_ident: &Ident,
    tree_root_absolute_path_parameter_ident: &Ident,
) -> Result<TokenStream, BrokenSymlinkEntryError> {
    // Note: the quoted `SymlinkDestinationType` and the one from the schema crate *are not the same type*.
    // The quoted one is `fs_more_test_harness::trees::SymlinkDestinationType`,
    // while the one we use here is `fs_more_test_harness_tree_schema::schema::SymlinkDestinationType`.
    let entry_symlink_destination_type = match symlink_destination_type {
        SymlinkDestinationType::File => quote! { SymlinkDestinationType::File },
        SymlinkDestinationType::Directory => quote! { SymlinkDestinationType::Directory },
    };

    Ok(quote! {
        self.#broken_symlink_path_variable_ident.assert_not_exists();
        self.#broken_symlink_destination_path_variable_ident.assert_not_exists();

        let absolute_destination_path =
            #tree_root_absolute_path_parameter_ident
                .join(&self.#broken_symlink_destination_path_variable_ident);

        initialize_symbolic_link(
            &self.#broken_symlink_path_variable_ident,
            &self.#broken_symlink_destination_path_variable_ident,
            #entry_symlink_destination_type,
        );

        self.#broken_symlink_path_variable_ident.assert_is_any_broken_symlink();
    })
}



#[derive(Debug, Clone)]
pub(crate) struct GeneratedBrokenSymlinkEntry {
    pub(crate) symlink_name: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) generated_code: TokenStream,
}


pub(crate) fn generate_code_for_broken_symlink_entry_in_tree(
    tree_root_struct_ident: &Ident,
    prepared_entry: PreparedBrokenSymlinkEntry,
) -> Result<GeneratedBrokenSymlinkEntry, BrokenSymlinkEntryError> {
    let broken_symlink_path_variable_ident = format_ident!("broken_symlink_path");
    let broken_symlink_destination_path_variable_ident =
        format_ident!("broken_symlink_destination_path");

    let tree_root_absolute_path_parameter_ident = format_ident!("tree_root_absolute_path");


    let broken_symlink_name = &prepared_entry.symlink_name;

    let broken_symlink_entry_struct_name_ident = &prepared_entry.struct_type_ident;
    let broken_symlink_destination_path_relative_to_tree_root =
        &prepared_entry.symlink_destination_relative_path;

    let broken_symlink_path_relative_to_tree_root =
        &prepared_entry.symlink_path_relative_to_tree_root;


    let documentation_for_broken_symlink_entry = format!(
        "This is a broken symbolic link entry. It resides at `{}` \n\
        and points to the non-existent location `{}`\n\
        (both paths are relative to the root of the test harness).\
        \n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        prepared_entry.symlink_path_relative_to_tree_root,
        broken_symlink_destination_path_relative_to_tree_root,
        tree_root_struct_ident
    );


    let generated_broken_symlink_post_initialization_code =
        construct_post_initializer_code_for_broken_symlink_entry(
            prepared_entry.symlink_destination_type,
            &broken_symlink_path_variable_ident,
            &broken_symlink_destination_path_variable_ident,
            &tree_root_absolute_path_parameter_ident,
        )?;


    let generated_code_for_broken_symlink_entry = quote! {
        #[doc = #documentation_for_broken_symlink_entry]
        pub struct #broken_symlink_entry_struct_name_ident {
            #broken_symlink_path_variable_ident: PathBuf,

            /// Symlink destination path, relative to the tree harness root.
            #broken_symlink_destination_path_variable_ident: PathBuf,
        }

        impl #broken_symlink_entry_struct_name_ident {
            #[track_caller]
            fn initialize(parent_directory_path: &Path) -> Self {
                let #broken_symlink_path_variable_ident = parent_directory_path.join(
                    #broken_symlink_name
                );

                let #broken_symlink_destination_path_variable_ident =
                    #broken_symlink_destination_path_relative_to_tree_root.into();


                #broken_symlink_path_variable_ident.assert_not_exists();

                Self {
                    #broken_symlink_path_variable_ident,
                    #broken_symlink_destination_path_variable_ident
                }
            }

            #[track_caller]
            fn post_initialize(&mut self, #tree_root_absolute_path_parameter_ident: &Path) {
                #generated_broken_symlink_post_initialization_code
            }
        }


        impl AsPath for #broken_symlink_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.#broken_symlink_path_variable_ident
            }
        }

        impl AsRelativePath for #broken_symlink_entry_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#broken_symlink_path_relative_to_tree_root)
            }
        }
    };


    Ok(GeneratedBrokenSymlinkEntry {
        symlink_name: prepared_entry.symlink_name,
        struct_type_ident: prepared_entry.struct_type_ident,
        documentation_for_parent_field: documentation_for_broken_symlink_entry,
        generated_code: generated_code_for_broken_symlink_entry,
    })
}
