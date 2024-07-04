use std::path::Path;

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::CodeGenerationContext;
use crate::schema::{FileDataConfiguration, FileEntry};



#[derive(Clone, Debug)]
pub struct PreparedFileEntry {
    pub(crate) entry_id: Option<String>,

    pub(crate) file_name: String,

    pub(crate) file_path_relative_to_tree_root: String,

    pub(crate) file_data: FileDataConfiguration,

    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,

    pub(crate) documentation_for_parent_field: String,

    pub(crate) documentation_for_struct: String,
}


pub(crate) fn prepare_file_entry(
    context: &mut CodeGenerationContext,
    tree_root_struct_ident: &Ident,
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


    let documentation_for_file_entry = format!(
        "This is a file residing at `{}` (relative to the root of the test harness).\
        \n\n<br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        file_relative_path_string, tree_root_struct_ident
    );



    PreparedFileEntry {
        entry_id: file.id.to_owned(),
        file_name: file_struct_name,
        file_path_relative_to_tree_root: file_relative_path_string,
        file_data: file
            .data
            .as_ref()
            .unwrap_or(&FileDataConfiguration::Empty)
            .to_owned(),
        struct_type_ident: file_struct_ident,
        preferred_parent_field_ident: preferred_field_name_on_parent,
        documentation_for_parent_field: documentation_for_file_entry.clone(),
        documentation_for_struct: documentation_for_file_entry,
    }
}


pub(crate) struct GeneratedFileEntry {
    // TODO remove
    /*
    pub(crate) struct_type_ident: Ident,

    pub(crate) preferred_parent_field_ident: Ident,

    pub(crate) documentation_for_parent_field: String, */
    pub(crate) generated_code: TokenStream,
}


fn construct_initializer_code_for_file_entry(
    prepared_file_entry: &PreparedFileEntry,
    state_at_initialization_variable_ident: &Ident,
) -> TokenStream {
    // Imports of e.g. `PathBuf`, `FileSystemHarnessFile`, `initialize_empty_file`, ...
    // must be taken care of at the top level (see `generate_rust_source_file_for_schema`).

    match &prepared_file_entry.file_data {
        FileDataConfiguration::Empty => quote! {
            path.assert_not_exists();
            initialize_empty_file(&path);
            path.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::Empty;
        },
        FileDataConfiguration::Text { content } => quote! {
            path.assert_not_exists();
            initialize_file_with_string(&path, #content);
            path.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::NonEmpty {
                content: Vec::from(#content.as_bytes())
            };
        },
        FileDataConfiguration::DeterministicRandom {
            seed,
            file_size_bytes,
        } => quote! {
            path.assert_not_exists();
            let binary_file_data = initialize_file_with_random_data(&path, #seed, #file_size_bytes);
            path.assert_is_file_and_not_symlink();

            let #state_at_initialization_variable_ident = FileState::NonEmpty {
                content: binary_file_data
            };
        },
    }
}


pub(crate) fn generate_code_for_file_entry_in_tree(
    _context: &CodeGenerationContext,
    _tree_root_struct_ident: &Ident,
    prepared_file_entry: PreparedFileEntry,
) -> GeneratedFileEntry {
    let state_at_initialization_variable_ident = format_ident!("state_at_initialization");

    let file_entry_initialization_code = construct_initializer_code_for_file_entry(
        &prepared_file_entry,
        &state_at_initialization_variable_ident,
    );

    let file_entry_struct_name_ident = &prepared_file_entry.struct_type_ident;
    let file_entry_documentation = &prepared_file_entry.documentation_for_struct;

    let file_path_relative_to_tree_root = &prepared_file_entry.file_path_relative_to_tree_root;


    let generated_code_for_file_entry = quote! {
        #[doc = #file_entry_documentation]
        pub struct #file_entry_struct_name_ident {
            file_path: PathBuf,
            #state_at_initialization_variable_ident: FileState,
        }

        impl #file_entry_struct_name_ident {
            #[track_caller]
            fn initialize<S>(parent_directory_path: &Path, file_name: S) -> Self
            where
                S: Into<String>
            {
                let file_path = parent_directory_path.join(file_name.into());

                #file_entry_initialization_code

                Self {
                    file_path,
                    #state_at_initialization_variable_ident
                }
            }
        }


        impl AsPath for #file_entry_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.file_path
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
        generated_code: generated_code_for_file_entry,
    }
}
