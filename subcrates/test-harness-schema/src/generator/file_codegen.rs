use std::path::Path;

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::NameCollisionAvoider;
use crate::schema::{FileDataConfiguration, FileSystemHarnessFileEntry};


pub(crate) struct GeneratedHarnessFileEntry {
    pub(crate) struct_type_name: String,

    pub(crate) preferred_parent_field_name: String,

    pub(crate) parent_field_documentation: String,

    pub(crate) generated_code: TokenStream,
}

pub(crate) fn codegen_harness_file_entry(
    struct_name_collision_avoider: &mut NameCollisionAvoider,
    root_harness_struct_ident: &Ident,
    parent_relative_path: &Path,
    file: &FileSystemHarnessFileEntry,
) -> GeneratedHarnessFileEntry {
    let friendly_upper_camel_case_file_name = file
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join("");

    let friendly_snake_case_field_name = file
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_");

    let file_struct_name = struct_name_collision_avoider.get_collision_free_name(
        &friendly_upper_camel_case_file_name
            .as_str()
            .to_upper_camel_case(),
    );

    let file_struct_ident = format_ident!("{}", file_struct_name);



    let file_relative_path = parent_relative_path.join(&file.name);
    let file_relative_path_string = file_relative_path
        .to_str()
        .expect("invalid relative file path: not UTF-8!");



    // Imports of e.g. `PathBuf` and `FileSystemHarnessFile` must be
    // taken care of at the top level (see `generate_rust_source_file_for_schema`).
    let generated_file_initialization_code =
        match file.data.clone().unwrap_or(FileDataConfiguration::Empty) {
            FileDataConfiguration::Empty => quote! {
                initialize_empty_file(&path);

                let initial_state = FileState::Empty;
            },
            FileDataConfiguration::Text { content } => quote! {
                initialize_file_with_string(&path, #content);

                let initial_state = FileState::NonEmpty {
                    content: Vec::from(#content.as_bytes())
                };
            },
            FileDataConfiguration::DeterministicRandom {
                seed,
                file_size_bytes,
            } => quote! {
                let binary_file_data =
                    initialize_file_with_random_data(&path, #seed, #file_size_bytes);

                let initial_state = FileState::NonEmpty {
                    content: binary_file_data
                };
            },
        };

    let generated_file_entry_comment = format!(
        "This is a file residing at `{}` (relative to the root of the test harness).\n\
        \n\
        <br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        parent_relative_path.join(&file.name).to_slash_lossy(),
        root_harness_struct_ident
    );

    let generated_file_entry_code = quote! {
        #[doc = #generated_file_entry_comment]
        pub struct #file_struct_ident {
            path: PathBuf,
            initial_state: FileState,
        }

        impl #file_struct_ident {
            fn new<S>(parent_path: PathBuf, file_name: S) -> Self
            where
                S: Into<String>,
            {
                let path = parent_path.join(file_name.into());
                path.assert_not_exists();

                #generated_file_initialization_code

                path.assert_is_file();

                Self {
                    path,
                    initial_state,
                }
            }
        }


        impl AsPath for #file_struct_ident {
            fn as_path(&self) -> &Path {
                &self.path
            }
        }

        impl CaptureableFilePath for #file_struct_ident {}


        impl AsInitialFileStateRef for #file_struct_ident {
            fn initial_state(&self) -> &FileState {
                &self.initial_state
            }
        }

        impl AssertableInitialFileCapture for #file_struct_ident {}

        impl AsRelativePath for #file_struct_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(#file_relative_path_string)
            }
        }
    };


    GeneratedHarnessFileEntry {
        struct_type_name: file_struct_name,
        preferred_parent_field_name: friendly_snake_case_field_name,
        parent_field_documentation: generated_file_entry_comment,
        generated_code: generated_file_entry_code,
    }
}
