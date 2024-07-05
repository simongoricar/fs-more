use fs_more_test_harness_tree_schema::schema::FileDataConfiguration;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::PreparedFileEntry;



#[derive(Debug, Clone)]
pub(crate) struct GeneratedFileEntry {
    pub(crate) file_name: String,

    pub(crate) struct_type_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) generated_code: TokenStream,
}


fn construct_initializer_code_for_file_entry(
    prepared_file_entry: &PreparedFileEntry,
    file_path_variable_ident: &Ident,
    state_at_initialization_variable_ident: &Ident,
) -> TokenStream {
    // Imports of e.g. `PathBuf`, `FileSystemHarnessFile`, `initialize_empty_file`, ...
    // must be taken care of at the top level (see `generate_rust_source_file_for_schema`).

    match &prepared_file_entry.file_data {
        FileDataConfiguration::Empty => quote! {
            #file_path_variable_ident.assert_not_exists();

            initialize_empty_file(
                &#file_path_variable_ident
            );

            #file_path_variable_ident.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::Empty;
        },
        FileDataConfiguration::Text { content } => quote! {
            #file_path_variable_ident.assert_not_exists();

            initialize_file_with_string(
                &#file_path_variable_ident,
                #content
            );

            #file_path_variable_ident.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::NonEmpty {
                content: Vec::from(#content.as_bytes())
            };
        },
        FileDataConfiguration::DeterministicRandom {
            seed,
            file_size_bytes,
        } => quote! {
            #file_path_variable_ident.assert_not_exists();

            let binary_file_data = initialize_file_with_random_data(
                &#file_path_variable_ident,
                #seed,
                #file_size_bytes
            );

            #file_path_variable_ident.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::NonEmpty {
                content: binary_file_data
            };
        },
    }
}


pub(crate) fn generate_code_for_file_entry_in_tree(
    tree_root_struct_ident: &Ident,
    prepared_file_entry: PreparedFileEntry,
) -> GeneratedFileEntry {
    let file_path_variable_ident = format_ident!("file_path");
    let state_at_initialization_variable_ident = format_ident!("state_at_initialization");


    let documentation_for_file_entry = format!(
        "This is a file residing at `{}` (relative to the root of the tree).\
        \n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        prepared_file_entry.file_path_relative_to_tree_root, tree_root_struct_ident
    );

    let file_name = &prepared_file_entry.file_name;

    let file_entry_initialization_code = construct_initializer_code_for_file_entry(
        &prepared_file_entry,
        &file_path_variable_ident,
        &state_at_initialization_variable_ident,
    );

    let file_entry_struct_name_ident = &prepared_file_entry.struct_type_ident;
    let file_path_relative_to_tree_root = &prepared_file_entry.file_path_relative_to_tree_root;


    let generated_code_for_file_entry = quote! {
        #[doc = #documentation_for_file_entry]
        pub struct #file_entry_struct_name_ident {
            #file_path_variable_ident: PathBuf,
            #state_at_initialization_variable_ident: FileState,
        }

        impl #file_entry_struct_name_ident {
            #[track_caller]
            fn initialize(parent_directory_path: &Path) -> Self
            {
                let #file_path_variable_ident = parent_directory_path.join(#file_name);

                #file_entry_initialization_code

                Self {
                    #file_path_variable_ident,
                    #state_at_initialization_variable_ident
                }
            }
        }


        impl AsPath for #file_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.#file_path_variable_ident
            }
        }

        impl AsRelativePath for #file_entry_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#file_path_relative_to_tree_root)
            }
        }


        impl AsInitialFileStateRef for #file_entry_struct_name_ident {
            fn initial_state(&self) -> &FileState {
                &self.#state_at_initialization_variable_ident
            }
        }

        impl AssertableInitialFileCapture for #file_entry_struct_name_ident {}

        impl CaptureableFilePath for #file_entry_struct_name_ident {}
    };


    GeneratedFileEntry {
        file_name: prepared_file_entry.file_name,
        struct_type_ident: prepared_file_entry.struct_type_ident,
        documentation_for_parent_field: documentation_for_file_entry,
        generated_code: generated_code_for_file_entry,
    }
}
