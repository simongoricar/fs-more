use std::{
    fs::OpenOptions,
    io::{self, prelude::Write, BufWriter},
    path::{Path, PathBuf},
};

use documentation::construct_documentation;
use entry_generation::generate_code_for_all_tree_sub_entries;
use entry_preparation::{
    construct_field_initializer_code,
    construct_field_post_initializer_code,
    prepare_tree_entries,
};
use heck::ToUpperCamelCase;
use quote::{format_ident, quote};
use thiserror::Error;

use super::{directory_entry::DirectoryEntryError, symlink_entry::SymlinkEntryError};
use crate::{
    codegen::{CodeGenerationContext, PreparedEntryRegistry},
    name_collision::NameCollisionAvoider,
    schema::FileSystemHarnessSchema,
};


mod documentation;
mod entry_generation;
mod entry_preparation;



#[derive(Debug, Error)]
pub enum SchemaCodeGenerationError {
    #[error(
        "failed to generate code for directory entry {}",
        .directory_relative_path.display()
    )]
    DirectoryEntryError {
        #[source]
        error: DirectoryEntryError,

        directory_relative_path: PathBuf,
    },

    #[error(
        "failed to generate code for symlink entry {}",
        .symlink_relative_path.display()
    )]
    SymlinkEntryError {
        #[source]
        error: SymlinkEntryError,

        symlink_relative_path: PathBuf,
    },

    #[error("failed to parse generated source file")]
    SynParsingError(
        #[from]
        #[source]
        syn::Error,
    ),

    #[error(
        "failed to open or write to output file: {}",
        .output_file_path.display()
    )]
    FailedToWriteOutput {
        #[source]
        io_error: io::Error,

        output_file_path: PathBuf,
    },
}




pub fn generate_rust_source_file_for_schema(
    input_schema_file_path: &Path,
    schema: FileSystemHarnessSchema,
    output_file_path: &Path,
    overwrite_existing_file: bool,
) -> Result<(), SchemaCodeGenerationError> {
    let schema_file_name = input_schema_file_path
        .file_name()
        .expect("invalid schema file path, no file name")
        .to_str()
        .expect("invalid schema file path, not UTF-8");


    let tree_root_struct_name = schema.name.as_str().to_upper_camel_case();
    let tree_root_struct_name_ident = format_ident!("{}", tree_root_struct_name);

    let temporary_directory_path_ident = format_ident!("temporary_directory_path");
    let new_self_variable_ident = format_ident!("new_self");

    let tree_root_relative_path = Path::new(".");


    let global_struct_name_collision_avoider = NameCollisionAvoider::new_empty();
    let global_prepared_entry_registry = PreparedEntryRegistry::new();

    let mut global_context = CodeGenerationContext {
        struct_name_collision_avoider: global_struct_name_collision_avoider,
        prepared_entry_registry: global_prepared_entry_registry,
    };




    let prepared_entries =
        prepare_tree_entries(&schema, &mut global_context, tree_root_relative_path)?;


    let tree_struct_field_initializer_code =
        construct_field_initializer_code(&prepared_entries, &temporary_directory_path_ident);


    let potential_post_initialization_code =
        construct_field_post_initializer_code(&prepared_entries);



    let generation_output = generate_code_for_all_tree_sub_entries(
        &global_context,
        &tree_root_struct_name_ident,
        prepared_entries,
    )?;


    let generated_documentation = construct_documentation(
        &schema,
        &generation_output.generated_entries,
        schema_file_name,
        &tree_root_struct_name,
    );

    let module_documentation = &generated_documentation.module_documentation;
    let tree_root_struct_documentation = &generated_documentation.tree_root_struct_documentation;


    let token_stream_to_prepend_to_output = &generation_output.code_to_prepend;
    let tree_struct_fields = &generation_output.struct_field_specifiers;
    let tree_struct_field_names = &generation_output.struct_field_names;



    let self_construction_plus_potential_post_initialization_plus_return =
        if potential_post_initialization_code.is_some() {
            quote! {
                let mut #new_self_variable_ident = Self {
                    temporary_directory,
                    #(#tree_struct_field_names),*
                };

                #new_self_variable_ident.post_initialize();

                // Returns self, optionally post-initialized if required
                // (indicated by a post_initialize call between declaration and return).
                #new_self_variable_ident
            }
        } else {
            quote! {
                Self {
                    temporary_directory,
                    #(#tree_struct_field_names),*
                }
            }
        };

    let tree_struct_post_initialize_impl =
        potential_post_initialization_code.map(|post_initialization_code| {
            quote! {
                impl #tree_root_struct_name_ident {
                    fn post_initialize(&mut self) {
                        #post_initialization_code
                    }
                }
            }
        });



    let generated_module_code = quote! {
        use std::fs;
        use std::path::{Path, PathBuf};
        use tempfile::TempDir;

        use crate::prelude::*;

        use crate::trees::{
            initialize_empty_file,
            initialize_file_with_string,
            initialize_file_with_random_data,
            initialize_symbolic_link,
            SymlinkDestinationType,
            AsInitialFileStateRef
        };

        use fs_more_test_harness_generator::schema::FileDataConfiguration;


        #token_stream_to_prepend_to_output

        #[doc = #tree_root_struct_documentation]
        pub struct #tree_root_struct_name_ident {
            temporary_directory: TempDir,

            #(#tree_struct_fields),*
        }


        impl FileSystemHarness for #tree_root_struct_name_ident {
            #[track_caller]
            fn initialize() -> Self {
                let temporary_directory = tempfile::tempdir()
                    .expect("failed to initialize temporary directory");

                let #temporary_directory_path_ident = temporary_directory.path();
                #temporary_directory_path_ident.assert_is_directory_and_empty();

                #tree_struct_field_initializer_code

                #self_construction_plus_potential_post_initialization_plus_return
            }

            #[track_caller]
            fn destroy(self) {
                if self.temporary_directory.path().exists() {
                    self.temporary_directory
                        .close()
                        .expect("failed to destroy filesystem harness directory");
                } else {
                    println!(
                        "Temporary directory \"{}\" doesn't exist, no need to clean up.",
                        self.temporary_directory.path().display()
                    );
                }
            }
        }

        #tree_struct_post_initialize_impl


        impl AsPath for #tree_root_struct_name_ident {
            fn as_path(&self) -> &Path {
                self.temporary_directory.path()
            }
        }

        impl AsRelativePath for #tree_root_struct_name_ident {
            fn as_path_relative_to_harness_root(&self) -> &Path {
                Path::new(".")
            }
        }

        impl FileSystemHarnessDirectory for #tree_root_struct_name_ident {}
    };




    let file_without_top_comment = syn::parse_file(&generated_module_code.to_string())?;
    let formatted_file_without_top_comment = prettyplease::unparse(&file_without_top_comment);



    let mut buffered_file = {
        let file = match overwrite_existing_file {
            true => OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output_file_path)
                .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
                    io_error,
                    output_file_path: output_file_path.to_path_buf(),
                })?,
            false => OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output_file_path)
                .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
                    io_error,
                    output_file_path: output_file_path.to_path_buf(),
                })?,
        };

        BufWriter::new(file)
    };


    buffered_file
        .write_all(module_documentation.as_bytes())
        .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
            io_error,
            output_file_path: output_file_path.to_path_buf(),
        })?;

    buffered_file
        .write_all("\n\n\n".as_bytes())
        .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
            io_error,
            output_file_path: output_file_path.to_path_buf(),
        })?;

    buffered_file
        .write_all(formatted_file_without_top_comment.as_bytes())
        .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
            io_error,
            output_file_path: output_file_path.to_path_buf(),
        })?;


    let mut file = buffered_file.into_inner().map_err(|error| {
        SchemaCodeGenerationError::FailedToWriteOutput {
            io_error: error.into_error(),
            output_file_path: output_file_path.to_path_buf(),
        }
    })?;

    file.flush()
        .map_err(|io_error| SchemaCodeGenerationError::FailedToWriteOutput {
            io_error,
            output_file_path: output_file_path.to_path_buf(),
        })?;


    Ok(())
}
