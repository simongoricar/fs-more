use std::path::Path;

use heck::{ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use path_slash::PathBufExt;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::{file_codegen::codegen_harness_file_entry, NameCollisionAvoider};
use crate::schema::{FileSystemHarnessDirectoryEntry, FileSystemHarnessEntry};


pub(crate) struct GeneratedHarnessDirectoryEntry {
    pub(crate) struct_type_name: String,

    pub(crate) preferred_parent_field_name: String,

    pub(crate) parent_field_documentation: String,

    /// Includes the concatenated code from all descendant entries.
    pub(crate) generated_code: TokenStream,
}


pub(crate) fn codegen_harness_directory_entry(
    struct_name_collision_avoider: &mut NameCollisionAvoider,
    root_harness_struct_ident: &Ident,
    parent_relative_path: &Path,
    directory: &FileSystemHarnessDirectoryEntry,
) -> GeneratedHarnessDirectoryEntry {
    let friendly_upper_camel_case_directory_name = directory
        .name
        .split('.')
        .map(|chunk| chunk.to_upper_camel_case())
        .join("");

    let friendly_snake_case_directory_name = directory
        .name
        .split('.')
        .map(|chunk| chunk.to_snake_case())
        .join("_");


    let directory_struct_name = struct_name_collision_avoider.get_collision_free_name(
        &friendly_upper_camel_case_directory_name
            .as_str()
            .to_upper_camel_case(),
    );

    let directory_struct_name_ident = format_ident!("{}", directory_struct_name);


    let directory_relative_path = parent_relative_path.join(&directory.name);


    let mut struct_field_name_collision_avoider = NameCollisionAvoider::new_empty();
    let mut token_stream_to_prepend = TokenStream::new();


    let directory_entries = directory.entries.clone().unwrap_or_default();



    struct AnnotatedField {
        struct_field_name: String,
        struct_field_type: String,
    }

    let mut generated_struct_fields = Vec::with_capacity(directory_entries.len());
    let mut generated_field_names = Vec::with_capacity(directory_entries.len());
    let mut generated_field_initializers = Vec::with_capacity(directory_entries.len());
    let mut generated_annotated_fields = Vec::with_capacity(directory_entries.len());

    for entry in &directory_entries {
        match entry {
            FileSystemHarnessEntry::File(file_entry) => {
                let generated_file_entry = codegen_harness_file_entry(
                    struct_name_collision_avoider,
                    root_harness_struct_ident,
                    &directory_relative_path,
                    file_entry,
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
                    let #field_name_ident = <#field_type_ident>::new(directory_path.clone(), #file_name);
                });
            }
            FileSystemHarnessEntry::Directory(directory_entry) => {
                let generated_dir_entry = codegen_harness_directory_entry(
                    struct_name_collision_avoider,
                    root_harness_struct_ident,
                    &directory_relative_path,
                    directory_entry,
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


                let dir_name = &directory_entry.name;

                generated_field_initializers.push(quote! {
                    let #field_name_ident = <#field_type_ident>::new(directory_path.clone(), #dir_name);
                });
            }
        }
    }


    let generated_directory_struct_fields_list = {
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
            "This directory has the following entries:\n\
            {}",
            field_list
        )
    };

    let generated_directory_entry_comment = format!(
        "This is a sub-directory residing at `{}` (relative to the root of the test harness).\n\
        \n\
        {}\n\
        \n\
        <br>\n\n\
        <sup>This entry is part of the [`{}`] test harness tree.</sup>",
        parent_relative_path.join(&directory.name).to_slash_lossy(),
        generated_directory_struct_fields_list,
        root_harness_struct_ident
    );


    let generated_directory_entry_code = quote! {
        #token_stream_to_prepend

        #[doc = #generated_directory_entry_comment]
        pub struct #directory_struct_name_ident {
            directory_path: PathBuf,

            #(#generated_struct_fields),*
        }

        impl #directory_struct_name_ident {
            fn new<S>(parent_path: PathBuf, directory_name: S) -> Self
            where
                S: Into<String>,
            {
                let directory_path = parent_path.join(directory_name.into());

                directory_path.assert_not_exists();
                fs::create_dir(&directory_path).expect("failed to create directory");
                directory_path.assert_is_directory_and_empty();


                #(#generated_field_initializers)*

                Self {
                    directory_path,
                    #(#generated_field_names),*
                }
            }
        }

        impl AsPath for #directory_struct_name_ident {
            fn as_path(&self) -> &Path {
                &self.directory_path
            }
        }
    };


    GeneratedHarnessDirectoryEntry {
        struct_type_name: directory_struct_name,
        preferred_parent_field_name: friendly_snake_case_directory_name,
        parent_field_documentation: generated_directory_entry_comment,
        generated_code: generated_directory_entry_code,
    }
}
