mod name_collision;
use std::{
    fs::OpenOptions,
    io::{prelude::Write, BufWriter},
    path::Path,
};

use directory_codegen::codegen_harness_directory_entry;
use file_codegen::codegen_harness_file_entry;
use heck::ToUpperCamelCase;
use itertools::Itertools;
use miette::{Context, IntoDiagnostic, Result};
pub use name_collision::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::schema::{FileSystemHarnessEntry, FileSystemHarnessSchema};
mod directory_codegen;
mod file_codegen;




pub fn generate_rust_source_file_for_schema(
    input_schema_file_path: &Path,
    schema: FileSystemHarnessSchema,
    output_file_path: &Path,
    overwrite_existing_file: bool,
) -> Result<()> {
    let input_schema_file_name = input_schema_file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("UNKNOWN");

    let root_tree_struct_name = schema.name.as_str().to_upper_camel_case();
    let root_tree_struct_ident = format_ident!("{}", root_tree_struct_name);


    let mut global_struct_name_collision_avoider = NameCollisionAvoider::new_empty();

    let mut struct_field_name_collision_avoider = NameCollisionAvoider::new_empty();
    let mut token_stream_to_prepend = TokenStream::new();


    struct AnnotatedField {
        struct_field_name: String,
        struct_field_type: String,
    }

    let mut generated_struct_fields = Vec::with_capacity(schema.structure.entries.len());
    let mut generated_field_names = Vec::with_capacity(schema.structure.entries.len());
    let mut generated_field_initializers = Vec::with_capacity(schema.structure.entries.len());
    let mut generated_annotated_fields = Vec::with_capacity(schema.structure.entries.len());

    for entry in schema.structure.entries {
        match entry {
            FileSystemHarnessEntry::File(file_entry) => {
                let generated_file_entry = codegen_harness_file_entry(
                    &mut global_struct_name_collision_avoider,
                    &root_tree_struct_ident,
                    Path::new("."),
                    &file_entry,
                );


                let generated_code = generated_file_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };


                let field_name = struct_field_name_collision_avoider
                    .get_collision_free_name(&generated_file_entry.preferred_parent_field_name);
                let field_name_ident = format_ident!("{}", field_name);

                let field_type = generated_file_entry.struct_type_name;
                let field_type_ident = format_ident!("{}", field_type);

                let field_comment = generated_file_entry.parent_field_documentation;


                generated_struct_fields.push(quote! {
                    #[doc = #field_comment]
                    pub #field_name_ident: #field_type_ident
                });

                generated_field_names.push(quote! { #field_name_ident });

                generated_annotated_fields.push(AnnotatedField {
                    struct_field_name: field_name,
                    struct_field_type: field_type,
                });


                let file_name = &file_entry.name;

                generated_field_initializers.push(quote! {
                    let #field_name_ident = <#field_type_ident>::new(temporary_directory_path.to_owned(), #file_name);
                });
            }
            FileSystemHarnessEntry::Directory(dir_entry) => {
                let generated_dir_entry = codegen_harness_directory_entry(
                    &mut global_struct_name_collision_avoider,
                    &root_tree_struct_ident,
                    Path::new("."),
                    &dir_entry,
                );


                let generated_code = generated_dir_entry.generated_code;

                token_stream_to_prepend = quote! {
                    #token_stream_to_prepend
                    #generated_code
                };


                let field_name = struct_field_name_collision_avoider
                    .get_collision_free_name(&generated_dir_entry.preferred_parent_field_name);
                let field_name_ident = format_ident!("{}", field_name);

                let field_type = generated_dir_entry.struct_type_name;
                let field_type_ident = format_ident!("{}", field_type);

                let field_comment = generated_dir_entry.parent_field_documentation;


                generated_struct_fields.push(quote! {
                    #[doc = #field_comment]
                    pub #field_name_ident: #field_type_ident
                });

                generated_field_names.push(quote! { #field_name_ident });

                generated_annotated_fields.push(AnnotatedField {
                    struct_field_name: field_name,
                    struct_field_type: field_type,
                });


                let dir_name = &dir_entry.name;

                generated_field_initializers.push(quote! {
                    let #field_name_ident = <#field_type_ident>::new(temporary_directory_path.to_owned(), #dir_name);
                });
            }
        }
    }



    let generated_description = if let Some(description) = schema.description {
        format!(
            "//! \n\
            //! {}\n\
            //!",
            description
        )
    } else {
        "//!".to_string()
    };

    let generated_module_preamble = format!(
        "\
//! @generated
//! 
//! This code was automatically generated from \"{}\",
//! a file that describes this filesystem tree harness for testing.
{}
//! DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the 
//! test-harness-schema crate).
    
#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]\
        ",
        input_schema_file_name, generated_description
    );

    let root_tree_struct_fields_list = {
        let field_list = generated_annotated_fields
            .iter()
            .map(|field| {
                format!(
                    "- `{}` (see [`{}`])",
                    field.struct_field_name, field.struct_field_type
                )
            })
            .join("\n");

        format!(
            "This harness has the following entries at the top level:\n\
            {}",
            field_list
        )
    };

    let root_tree_struct_comment = format!(
        "A fs-more filesystem testing harness. Upon calling [`Self::initialize`],\n\
        it sets up a temporary directory and initializes the entire configured file tree.\n\
        When it's dropped or when [`Self::destroy`] is called, the temporary directory is removed.\n\
        \n\
        In addition to initializing the configured files and directories, a snapshot (\"capture\")\n\
        is created for each file. This is the same as [`CaptureableFilePath::capture_with_content`],\
        but the snapshot is created as tree initialization\n\
        \n\
        {}\n\
        \n\
        <br>\n\n\
        <sup>This tree and related code was automatically generated from the structure described in `{}`.</sup>",
        root_tree_struct_fields_list,
        input_schema_file_name
    );



    let generated_code = quote! {
        use std::fs;
        use std::path::{PathBuf, Path};
        use tempfile::TempDir;

        use crate::tree_framework::FileSystemHarness;
        use crate::tree_framework::AsInitialFileStateRef;
        use crate::tree_framework::AssertableInitialFileCapture;
        use crate::tree_framework::initialize_empty_file;
        use crate::tree_framework::initialize_file_with_string;
        use crate::tree_framework::initialize_file_with_random_data;

        use crate::assertable::AsPath;
        use crate::assertable::r#trait::AssertablePath;
        use crate::assertable::r#trait::CaptureableFilePath;
        use crate::assertable::file_capture::CapturedFileState;
        use crate::assertable::file_capture::FileState;

        use fs_more_test_harness_schema::schema::FileDataConfiguration;

        #token_stream_to_prepend

        #[doc = #root_tree_struct_comment]
        pub struct #root_tree_struct_ident {
            temporary_directory: TempDir,

            #(#generated_struct_fields),*
        }

        impl FileSystemHarness for #root_tree_struct_ident {
            fn initialize() -> Self {
                let temporary_directory =
                    tempfile::tempdir()
                        .expect("failed to initialize temporary directory");

                let temporary_directory_path = temporary_directory.path();
                temporary_directory_path.assert_is_directory_and_empty();


                #(#generated_field_initializers)*

                Self {
                    temporary_directory,

                    #(#generated_field_names),*
                }
            }

            fn destroy(self) {
                self.temporary_directory
                    .close()
                    .expect("failed to destroy filesystem harness directory");
            }
        }

        impl AsPath for #root_tree_struct_ident {
            fn as_path(&self) -> &Path {
                self.temporary_directory.path()
            }
        }
    };



    let file_without_initial_comment = syn::parse_file(&generated_code.to_string())
        .into_diagnostic()
        .wrap_err("Failed to parse generated code as syn::File.")?;

    let formatted_file = prettyplease::unparse(&file_without_initial_comment);

    let mut buffered_file = {
        let file = match overwrite_existing_file {
            true => OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output_file_path)
                .into_diagnostic()?,
            false => OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output_file_path)
                .into_diagnostic()?,
        };

        BufWriter::new(file)
    };

    buffered_file
        .write_all(generated_module_preamble.as_bytes())
        .into_diagnostic()?;

    buffered_file
        .write_all("\n\n\n".as_bytes())
        .into_diagnostic()?;

    buffered_file
        .write_all(formatted_file.as_bytes())
        .into_diagnostic()?;

    let mut file = buffered_file.into_inner().into_diagnostic()?;
    file.flush().into_diagnostic()?;


    Ok(())
}
