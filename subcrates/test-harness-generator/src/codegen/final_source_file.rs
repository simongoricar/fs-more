use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{self, prelude::Write, BufWriter},
    path::{Path, PathBuf},
};

use heck::ToUpperCamelCase;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use thiserror::Error;

use super::{
    directory_in_tree::{generate_code_for_directory_entry_in_tree, DirectoryEntryError},
    file_in_tree::generate_code_for_file_entry_in_tree,
    symlink_in_tree::{generate_code_for_symlink_entry_in_tree, SymlinkEntryError},
};
use crate::{
    codegen::{
        directory_in_tree::prepare_directory_entry,
        file_in_tree::prepare_file_entry,
        symlink_in_tree::prepare_symlink_entry,
        AnyPreparedEntry,
        CodeGenerationContext,
        PreparedEntryRegistry,
    },
    name_collision::NameCollisionAvoider,
    schema::{FileDataConfiguration, FileSystemHarnessEntry, FileSystemHarnessSchema},
};



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


fn prepend_lines_with_inner_line_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| format!("//! {}", line))
        .join("\n")
}



fn format_tree_structure_as_string(schema: &FileSystemHarnessSchema) -> String {
    let mut formatted_lines = vec![".".to_string()];


    struct PendingEntry<'s> {
        entry: &'s FileSystemHarnessEntry,
        depth: usize,
    }

    let mut depth_first_queue = VecDeque::new();
    depth_first_queue.extend(schema.structure.entries.iter().map(|first_level_entry| {
        PendingEntry {
            entry: first_level_entry,
            depth: 1,
        }
    }));


    while let Some(next_item) = depth_first_queue.pop_front() {
        let mut formatted_line = String::new();

        for _ in 0..(next_item.depth.saturating_sub(1)) {
            formatted_line.push_str("|   ");
        }

        if next_item.depth > 0 {
            formatted_line.push_str("|-- ");
        }

        formatted_line.push_str(
            match next_item.entry {
                FileSystemHarnessEntry::File(file) => {
                    let file_description =
                        match file.data.as_ref().unwrap_or(&FileDataConfiguration::Empty) {
                            FileDataConfiguration::Empty => "empty".to_string(),
                            FileDataConfiguration::Text { content } => {
                                let human_size =
                                    humansize::format_size(content.len(), humansize::BINARY);

                                format!("text data, {}", human_size)
                            }
                            FileDataConfiguration::DeterministicRandom {
                                file_size_bytes, ..
                            } => {
                                let human_size =
                                    humansize::format_size(*file_size_bytes, humansize::BINARY);

                                format!("random data, {}", human_size)
                            }
                        };

                    format!("{} ({})", file.name.as_str(), file_description)
                }
                FileSystemHarnessEntry::Directory(directory) => directory.name.to_string(),
                FileSystemHarnessEntry::Symlink(symlink) => symlink.name.to_string(),
            }
            .as_str(),
        );

        formatted_lines.push(formatted_line);


        if let FileSystemHarnessEntry::Directory(directory_entry) = next_item.entry {
            if let Some(directory_entries) = directory_entry.entries.as_ref() {
                for sub_entry in directory_entries {
                    depth_first_queue.push_front(PendingEntry {
                        entry: sub_entry,
                        depth: next_item.depth + 1,
                    });
                }
            }
        }
    }


    format!(
        "```md\n\
        {}\n\
        ```",
        formatted_lines.join("\n")
    )
}



pub(crate) struct DocumentationPieces {
    module_documentation: String,
    tree_root_struct_documentation: String,
}

pub(crate) fn construct_documentation(
    schema: &FileSystemHarnessSchema,
    prepared_entries: &[AnyPreparedEntry],
    schema_file_name: &str,
    tree_root_struct_name: &str,
) -> DocumentationPieces {
    let visually_formatted_file_tree = format_tree_structure_as_string(schema);


    let custom_schema_description = if let Some(description) = schema.description.as_ref() {
        format!(
            "//! \n\
            //! {}\n\
            //!",
            description
        )
    } else {
        "//!".to_string()
    };


    let module_documentation = format!(
        "\
//! @generated
//!
//! This code was automatically generated from \"{}\",
//! a file that describes this filesystem tree harness for testing.
{}
//!
//! The full file tree is as follows:
{}
//!
//! <sup>DO NOT MODIFY THIS FILE. INSTEAD, MODIFY THE SOURCE JSON DATA FILE,
//! AND REGENERATE THIS FILE (see the CLI provided by the
//! test-harness-schema crate).</sup>

#![allow(unused_imports)]
#![allow(clippy::disallowed_names)]
#![allow(dead_code)]\
        ",
        schema_file_name,
        custom_schema_description,
        prepend_lines_with_inner_line_comments(&visually_formatted_file_tree)
    );



    let formatted_struct_field_list = prepared_entries
        .iter()
        .map(|entry| {
            let (field_name, field_type_ident) = match entry {
                AnyPreparedEntry::Directory {
                    entry,
                    actual_field_name_ident_on_parent,
                } => (actual_field_name_ident_on_parent, &entry.struct_type_ident),
                AnyPreparedEntry::File {
                    entry,
                    actual_field_name_ident_on_parent,
                } => (actual_field_name_ident_on_parent, &entry.struct_type_ident),
                AnyPreparedEntry::Symlink {
                    entry,
                    actual_field_name_ident_on_parent,
                } => (actual_field_name_ident_on_parent, &entry.struct_type_ident),
            };

            format!("- `{}` (see [`{}`])", field_name, field_type_ident)
        })
        .join("\n");

    let tree_root_struct_documentation = format!(
        "`fs-more` filesystem tree for testing. Upon calling [`{}::initialize`],\n\
        a temporary directory is set up, and the entire pre-defined filesystem tree is initialized.\n\
        When [`{}::destroy`] is called (or when the struct is dropped), the temporary directory is removed,\n\
        along with all of its contents.\n\
        \n\
        In addition to initializing the configured files and directories, a snapshot is created\n\
        for each file (also called a \"capture\"). This is the same as [`CaptureableFilePath::capture_with_content`],\
        but the snapshot is recorded at tree initialization.\n\
        \n\
        This harness has the following sub-entries at the top level (files, sub-directories, ...):\n\
        {}\n\
        \n\n\
        The full file tree is as follows:\n\
        {}\n\
        \n\n\
        <br>\n\n\
        <sup>This tree and related code was automatically generated from the structure described in `{}`.</sup>",
        tree_root_struct_name,
        tree_root_struct_name,
        formatted_struct_field_list,
        visually_formatted_file_tree,
        schema_file_name
    );


    DocumentationPieces {
        module_documentation,
        tree_root_struct_documentation,
    }
}


pub(crate) fn construct_field_initializers(
    prepared_entries: &[AnyPreparedEntry],
    temporary_directory_path_variable_ident: &Ident,
) -> TokenStream {
    let mut individual_initializers = Vec::with_capacity(prepared_entries.len());

    for entry in prepared_entries {
        let initializer = match entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = &entry.struct_type_ident;
                let directory_name_string_value = &entry.directory_name;

                quote! {
                    let #actual_field_name_ident_on_parent = <#field_type_ident>::initialize(
                        &#temporary_directory_path_variable_ident,
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
                        &#temporary_directory_path_variable_ident,
                        #file_name_string_value,
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
                        &#temporary_directory_path_variable_ident,
                        #symlink_name_string_value
                    );
                }
            }
        };

        individual_initializers.push(initializer);
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(individual_initializers);
    final_token_stream
}



pub(crate) fn construct_field_post_initializers(
    prepared_entries: &[AnyPreparedEntry],
) -> Option<TokenStream> {
    let mut individual_initializers = Vec::with_capacity(prepared_entries.len());

    for entry in prepared_entries {
        let initializer = match entry {
            AnyPreparedEntry::Directory {
                actual_field_name_ident_on_parent,
                ..
            } => {
                quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize();
                }
            }
            AnyPreparedEntry::Symlink {
                actual_field_name_ident_on_parent,
                ..
            } => {
                quote! {
                    self.#actual_field_name_ident_on_parent.post_initialize();
                }
            }
            AnyPreparedEntry::File { .. } => continue,
        };

        individual_initializers.push(initializer);
    }


    if individual_initializers.is_empty() {
        return None;
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(individual_initializers);
    Some(final_token_stream)
}



pub(crate) fn prepare_tree_entries(
    schema: &FileSystemHarnessSchema,
    global_context: &mut CodeGenerationContext,
    tree_root_struct_name_ident: &Ident,
    tree_root_relative_path: &Path,
) -> Result<Vec<AnyPreparedEntry>, SchemaCodeGenerationError> {
    let mut local_struct_field_name_collision_avoider = NameCollisionAvoider::new_empty();

    let mut prepared_entries = Vec::with_capacity(schema.structure.entries.len());
    for entry in &schema.structure.entries {
        match entry {
            FileSystemHarnessEntry::File(file_entry) => {
                let prepared_file_entry = prepare_file_entry(
                    global_context,
                    tree_root_struct_name_ident,
                    tree_root_relative_path,
                    file_entry,
                );

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_file_entry.preferred_parent_field_ident);


                prepared_entries.push(AnyPreparedEntry::File {
                    entry: prepared_file_entry,
                    actual_field_name_ident_on_parent,
                })
            }
            FileSystemHarnessEntry::Directory(directory_entry) => {
                let prepared_directory_entry = prepare_directory_entry(
                    global_context,
                    tree_root_struct_name_ident,
                    tree_root_relative_path,
                    directory_entry,
                )
                .map_err(|error| {
                    SchemaCodeGenerationError::DirectoryEntryError {
                        error,
                        directory_relative_path: tree_root_relative_path
                            .join(&directory_entry.name),
                    }
                })?;

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_directory_entry.preferred_parent_field_ident);


                prepared_entries.push(AnyPreparedEntry::Directory {
                    entry: prepared_directory_entry,
                    actual_field_name_ident_on_parent,
                })
            }
            FileSystemHarnessEntry::Symlink(symlink_entry) => {
                let prepared_symlink_entry = prepare_symlink_entry(
                    global_context,
                    tree_root_struct_name_ident,
                    tree_root_relative_path,
                    symlink_entry,
                )
                .map_err(|error| SchemaCodeGenerationError::SymlinkEntryError {
                    error,
                    symlink_relative_path: tree_root_relative_path.join(&symlink_entry.name),
                })?;

                let actual_field_name_ident_on_parent = local_struct_field_name_collision_avoider
                    .collision_free_ident(&prepared_symlink_entry.preferred_parent_field_ident);


                prepared_entries.push(AnyPreparedEntry::Symlink {
                    entry: prepared_symlink_entry,
                    actual_field_name_ident_on_parent,
                })
            }
        }
    }

    Ok(prepared_entries)
}



pub(crate) struct TreeEntriesGenerationOutput {
    code_to_prepend: TokenStream,
    struct_field_specifiers: Vec<TokenStream>,
    struct_field_names: Vec<TokenStream>,
}

pub(crate) fn generate_code_for_all_tree_sub_entries(
    context: &CodeGenerationContext,
    tree_root_struct_name_ident: &Ident,
    prepared_entries: Vec<AnyPreparedEntry>,
) -> Result<TreeEntriesGenerationOutput, SchemaCodeGenerationError> {
    let mut code_chunks_to_prepend = Vec::with_capacity(prepared_entries.len());

    let mut struct_field_specifiers = Vec::with_capacity(prepared_entries.len());
    let mut struct_field_names = Vec::with_capacity(prepared_entries.len());


    for prepared_entry in prepared_entries {
        let (generated_code, struct_field_specifier, struct_field_name) = match prepared_entry {
            AnyPreparedEntry::Directory {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();
                let directory_path_relative_to_tree_root =
                    entry.directory_path_relative_to_tree_root.clone();


                let generated_directory_entry = generate_code_for_directory_entry_in_tree(
                    context,
                    tree_root_struct_name_ident,
                    entry,
                )
                .map_err(|error| {
                    SchemaCodeGenerationError::DirectoryEntryError {
                        error,
                        directory_relative_path: directory_path_relative_to_tree_root.into(),
                    }
                })?;

                (
                    generated_directory_entry.generated_code,
                    quote! {
                        #actual_field_name_ident_on_parent: #field_type_ident
                    },
                    quote! {
                        #actual_field_name_ident_on_parent
                    },
                )
            }
            AnyPreparedEntry::File {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();

                let generated_directory_entry = generate_code_for_file_entry_in_tree(
                    context,
                    tree_root_struct_name_ident,
                    entry,
                );

                (
                    generated_directory_entry.generated_code,
                    quote! {
                        #actual_field_name_ident_on_parent: #field_type_ident
                    },
                    quote! {
                        #actual_field_name_ident_on_parent
                    },
                )
            }
            AnyPreparedEntry::Symlink {
                entry,
                actual_field_name_ident_on_parent,
            } => {
                let field_type_ident = entry.struct_type_ident.clone();
                let symlink_path_relative_to_tree_root =
                    entry.symlink_path_relative_to_tree_root.clone();


                let generated_directory_entry =
                    generate_code_for_symlink_entry_in_tree(context, entry).map_err(|error| {
                        SchemaCodeGenerationError::SymlinkEntryError {
                            error,
                            symlink_relative_path: symlink_path_relative_to_tree_root.into(),
                        }
                    })?;

                (
                    generated_directory_entry.generated_code,
                    quote! {
                        #actual_field_name_ident_on_parent: #field_type_ident
                    },
                    quote! {
                        #actual_field_name_ident_on_parent
                    },
                )
            }
        };


        code_chunks_to_prepend.push(generated_code);
        struct_field_specifiers.push(struct_field_specifier);
        struct_field_names.push(struct_field_name);
    }


    let mut final_token_stream = TokenStream::new();
    final_token_stream.extend(code_chunks_to_prepend);

    Ok(TreeEntriesGenerationOutput {
        code_to_prepend: final_token_stream,
        struct_field_names,
        struct_field_specifiers,
    })
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

    let tree_root_relative_path = Path::new(".");


    let global_struct_name_collision_avoider = NameCollisionAvoider::new_empty();
    let global_prepared_entry_registry = PreparedEntryRegistry::new();

    let mut global_context = CodeGenerationContext {
        struct_name_collision_avoider: global_struct_name_collision_avoider,
        prepared_entry_registry: global_prepared_entry_registry,
    };




    let prepared_entries = prepare_tree_entries(
        &schema,
        &mut global_context,
        &tree_root_struct_name_ident,
        tree_root_relative_path,
    )?;


    let generated_documentation = construct_documentation(
        &schema,
        &prepared_entries,
        schema_file_name,
        &tree_root_struct_name,
    );

    let module_documentation = &generated_documentation.module_documentation;
    let tree_root_struct_documentation = &generated_documentation.tree_root_struct_documentation;



    let tree_struct_field_initializer_code =
        construct_field_initializers(&prepared_entries, &temporary_directory_path_ident);


    let potential_tree_struct_field_post_initializer_code =
        construct_field_post_initializers(&prepared_entries);

    let potential_post_initialize_call =
        if potential_tree_struct_field_post_initializer_code.is_some() {
            Some(quote! {
                self.post_initialize();
            })
        } else {
            None
        };

    let tree_struct_post_initialize_impl = potential_tree_struct_field_post_initializer_code.map(
        |tree_struct_post_initializer_code| {
            quote! {
                impl #tree_root_struct_name_ident {
                    fn post_initialize(&mut self) {
                        #tree_struct_post_initializer_code
                    }
                }
            }
        },
    );



    let generated_entries = generate_code_for_all_tree_sub_entries(
        &global_context,
        &tree_root_struct_name_ident,
        prepared_entries,
    )?;

    let token_stream_to_prepend_to_output = &generated_entries.code_to_prepend;
    let tree_struct_fields = &generated_entries.struct_field_specifiers;
    let tree_struct_field_names = &generated_entries.struct_field_names;


    let generated_module_code = quote! {
        use std::fs;
        use std::path::{Path, PathBuf};
        use tempfile::TempDir;

        use crate::prelude::*;

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

                #potential_post_initialize_call

                Self {
                    temporary_directory,
                    #(#tree_struct_field_names),*
                }
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
