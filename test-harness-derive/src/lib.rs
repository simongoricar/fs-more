use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    DeriveInput,
    MetaNameValue,
    Token,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FieldPathType {
    Root,
    File,
    Directory,
}

#[allow(clippy::enum_variant_names)]
enum AnnotatedStructField {
    RootPath {
        field_name: syn::Ident,
    },
    FilePath {
        field_name: syn::Ident,
        file_path: syn::LitStr,
        file_contents: Option<syn::Expr>,
    },
    DirectoryPath {
        field_name: syn::Ident,
        directory_path: syn::LitStr,
    },
}

fn infer_field_path_type_from_field_type(
    field: &syn::Field,
) -> Option<FieldPathType> {
    let syn::Type::Path(field_value_type) = &field.ty else {
        return None;
    };

    let last_field_type_segment = field_value_type
        .path
        .segments
        .iter()
        .last()
        .expect("Expected at least one path segment in the field type.");

    if last_field_type_segment.ident.eq("AssertableRootPath") {
        Some(FieldPathType::Root)
    } else if last_field_type_segment.ident.eq("AssertableFilePath") {
        Some(FieldPathType::File)
    } else if last_field_type_segment.ident.eq("AssertableDirectoryPath") {
        Some(FieldPathType::Directory)
    } else {
        None
    }
}

fn parse_field(field: syn::Field) -> Option<AnnotatedStructField> {
    let path_type_inferred_from_field_type =
        infer_field_path_type_from_field_type(&field);

    let Some(field_name) = field.ident else {
        abort_call_site!(
            "Can derive this only on structs with named fields."
        );
    };

    for attribute in field.attrs {
        if !matches!(attribute.style, syn::AttrStyle::Outer) {
            continue;
        }

        match attribute.meta {
            syn::Meta::Path(path_annotation) => {
                let last_segment = path_annotation
                    .segments
                    .iter()
                    .last()
                    .unwrap_or_else(|| {
                        abort_call_site!("Invalid attribute: no last segment.")
                    });

                if last_segment.ident.eq("root") {
                    let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                        abort_call_site!(
                            "Field {} has the #[root] attribute, but isn't of a recognized assertable type: \n
                            expected AssertableRootPath, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    };

                    if *inferred_path_type != FieldPathType::Root {
                        abort_call_site!(
                            "Field {} has the #[root] attribute, but isn't of the correct type: \n
                            expected AssertableRootPath type, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    }

                    return Some(AnnotatedStructField::RootPath { field_name });
                }
            }
            syn::Meta::List(list_annotation) => {
                if list_annotation.path.is_ident("file") {
                    let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                        abort_call_site!(
                            "Field {} has the #[file(...)] attribute, but isn't of a recognized assertable type: \n
                            expected AssertableFilePath type, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    };

                    if *inferred_path_type != FieldPathType::File {
                        abort_call_site!(
                            "Field {} has the #[file(...)] attribute, but isn't of the correct type: \n
                            expected AssertableFilePath type, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    }

                    let file_subattributes: Punctuated<MetaNameValue, Token![,]> = list_annotation
                        .parse_args_with(Punctuated::parse_terminated)
                        .unwrap_or_else(|_| abort_call_site!(
                            "Expected a #[file(path = \"...\", contents = SOME_VEC_OF_U8)] \
                                attribute, got {} instead.",
                                list_annotation.to_token_stream().to_string()
                        ));

                    let mut path_subattribute: Option<syn::LitStr> = None;
                    let mut contents_subattribute: Option<syn::Expr> = None;

                    for subattribute in file_subattributes {
                        if subattribute.path.is_ident("path") {
                            let syn::Expr::Lit(literal) = subattribute.value else {
                                abort_call_site!(
                                    "Unexpected #[file(...)] value of field path \
                                    (expected string literal): {}",
                                    subattribute.to_token_stream().to_string(),
                                )
                            };

                            let syn::Lit::Str(string_literal) = literal.lit else {
                                abort_call_site!(
                                    "Unexpected #[file(...)] value of field path \
                                    (expected string literal): {}",
                                    literal.to_token_stream().to_string(),
                                );
                            };

                            path_subattribute = Some(string_literal);
                        } else if subattribute.path.is_ident("content") {
                            contents_subattribute = Some(subattribute.value);
                        } else {
                            abort_call_site!(
                                "Unexpected #[file(...)] field: {}",
                                subattribute.to_token_stream().to_string(),
                            );
                        }
                    }

                    let Some(path_subattribute) = path_subattribute else {
                        abort_call_site!(
                            "Missing path = ... key-value pair in #[file(...)] attribute."
                        );
                    };

                    return Some(AnnotatedStructField::FilePath {
                        field_name,
                        file_path: path_subattribute,
                        file_contents: contents_subattribute,
                    });
                } else if list_annotation.path.is_ident("directory") {
                    let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                        abort_call_site!(
                            "Field {} has the #[directory(...)] attribute, but isn't of a recognized assertable type: \n
                            expected AssertableDirectoryPath type, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    };

                    if *inferred_path_type != FieldPathType::Directory {
                        abort_call_site!(
                            "Field {} has the #[directory(...)] attribute, but isn't of the correct type: \n
                            expected AssertableDirectoryPath type, got {}.",
                            field_name.to_string(),
                            field.ty.to_token_stream().to_string(),
                        );
                    }


                    let dir_subattributes: Punctuated<MetaNameValue, Token![,]> = list_annotation.parse_args_with(Punctuated::parse_terminated)
                    .unwrap_or_else(|_| abort_call_site!(
                        "Expected a #[directory(path = \"...\")] attribute, got {} instead.",
                        list_annotation.to_token_stream().to_string()
                    ));


                    let mut path_subattribute: Option<syn::LitStr> = None;

                    for subattribute in dir_subattributes {
                        if subattribute.path.is_ident("path") {
                            let syn::Expr::Lit(literal) = subattribute.value else {
                                abort_call_site!(
                                    "Unexpected #[directory(...)] value of field path \
                                    (expected string literal): {}",
                                    subattribute.to_token_stream().to_string(),
                                )
                            };

                            let syn::Lit::Str(string_literal) = literal.lit else {
                                abort_call_site!(
                                    "Unexpected #[directory(...)] value of field path \
                                    (expected string literal): {}",
                                    literal.to_token_stream().to_string(),
                                );
                            };

                            path_subattribute = Some(string_literal);
                        } else {
                            abort_call_site!(
                                "Unexpected #[directory(...)] field: {}",
                                subattribute.to_token_stream().to_string(),
                            );
                        }
                    }

                    let Some(path_subattribute) = path_subattribute else {
                        abort_call_site!(
                            "Missing path = ... key-value pair in #[directory(...)] attribute."
                        );
                    };

                    return Some(AnnotatedStructField::DirectoryPath {
                        field_name,
                        directory_path: path_subattribute,
                    });
                }
            }
            _ => {}
        };
    }

    match path_type_inferred_from_field_type {
        Some(_) => {
            abort_call_site!(
                "Field {} is of recognized assertable type {}, but is missing \
                the associated #[root]/#[file(...)]/#[directory(...)] attribute.",
                field_name.to_string(),
                field.ty.to_token_stream().to_string(),
            )
        }
        None => None,
    }
}

fn construct_initialization_expression_for_struct_field(
    field: &AnnotatedStructField,
    temporary_dir_ident: syn::Ident,
) -> proc_macro2::TokenStream {
    match field {
        AnnotatedStructField::RootPath { field_name } => {
            quote! {
                let #field_name = AssertableRootPath::new(#temporary_dir_ident);
            }
        }
        AnnotatedStructField::FilePath {
            field_name,
            file_path,
            file_contents,
        } => {
            if let Some(contents) = file_contents {
                quote! {
                    let #field_name = {
                        let child_path = #temporary_dir_ident.child(#file_path);
                        let original_contents = &#contents;

                        child_path.write_binary(original_contents)?;

                        assert_eq!(
                            &std::fs::read(child_path.path()).unwrap(),
                            original_contents
                        );

                        AssertableFilePath::from_child_path(child_path, original_contents)
                    };
                }
            } else {
                quote! {
                    let #field_name = {
                        let child_path = #temporary_dir_ident.child(#file_path);

                        AssertableFilePath::from_child_path(child_path, Vec::with_capacity(0))
                    };
                }
            }
        }
        AnnotatedStructField::DirectoryPath {
            field_name,
            directory_path,
        } => {
            quote! {
                let #field_name = {
                    let child_path = #temporary_dir_ident.child(#directory_path);
                    child_path.create_dir_all()?;

                    AssertableDirectoryPath::from_child_path(child_path)
                };
            }
        }
    }
}


#[proc_macro_derive(FilesystemTreeHarness, attributes(root, file, directory))]
#[proc_macro_error]
pub fn fs_harness_tree(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    let struct_name = input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    let syn::Data::Struct(struct_data) = input.data else {
        abort_call_site!(
            "FilesystemTreeHarness can only be derived on structs."
        );
    };

    let syn::Fields::Named(named_fields) = struct_data.fields else {
        abort_call_site!(
            "FilesystemTreeHarness can only be derived on structs with named fields."
        );
    };

    let mut all_field_names: Vec<syn::Ident> = Vec::new();

    let mut root_field: Option<AnnotatedStructField> = None;
    let mut directory_fields: Vec<AnnotatedStructField> = Vec::new();
    let mut file_fields: Vec<AnnotatedStructField> = Vec::new();

    for field in named_fields.named {
        if let Some(parsed_field) = parse_field(field) {
            match &parsed_field {
                AnnotatedStructField::RootPath { field_name } => {
                    if root_field.is_some() {
                        abort_call_site!(
                            "Invalid annotations: expecting precisely \
                            one #[root]-annotated field, got {}."
                        );
                    }

                    all_field_names.push(field_name.to_owned());
                    root_field = Some(parsed_field);
                }
                AnnotatedStructField::FilePath { field_name, .. } => {
                    all_field_names.push(field_name.to_owned());
                    file_fields.push(parsed_field);
                }
                AnnotatedStructField::DirectoryPath { field_name, .. } => {
                    all_field_names.push(field_name.to_owned());
                    directory_fields.push(parsed_field);
                }
            }
        }
    }

    let root_field = root_field.unwrap_or_else(|| {
        abort_call_site!(
            "Invalid annotations: missing a #[root]-annotated field."
        )
    });

    let root_field_name = match &root_field {
        AnnotatedStructField::RootPath { field_name } => field_name.clone(),
        _ => panic!("BUG: `root_field` should be a RootPath?!"),
    };


    let initialization_method: proc_macro2::TokenStream = {
        let temporary_dir_variable_ident =
            syn::Ident::new("temp_directory", Span::call_site());


        let root_field_initialization_expr =
            construct_initialization_expression_for_struct_field(
                &root_field,
                temporary_dir_variable_ident.clone(),
            );

        let directory_fields_initialization_exprs = directory_fields
            .iter()
            .map(|field| {
                construct_initialization_expression_for_struct_field(
                    field,
                    temporary_dir_variable_ident.clone(),
                )
            })
            .collect::<Vec<_>>();

        let file_fields_initialization_exprs = file_fields
            .iter()
            .map(|field| {
                construct_initialization_expression_for_struct_field(
                    field,
                    temporary_dir_variable_ident.to_owned(),
                )
            })
            .collect::<Vec<_>>();

        quote! {
            pub fn new() -> Result<Self, assert_fs::fixture::FixtureError> {
                use assert_fs::fixture::PathChild;
                use assert_fs::fixture::FileWriteBin;
                use assert_fs::fixture::PathCreateDir;

                let #temporary_dir_variable_ident = assert_fs::TempDir::new()?;

                #(#directory_fields_initialization_exprs)*
                #(#file_fields_initialization_exprs)*
                #root_field_initialization_expr

                Ok(Self {
                    #(#all_field_names),*
                })
            }
        }
    };

    let destroy_method = quote! {
        pub fn destroy(self) -> Result<(), assert_fs::fixture::FixtureError> {
            let temp_dir = self.#root_field_name.into_temp_dir();
            temp_dir.close()?;

            Ok(())
        }
    };

    quote!(
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #initialization_method
            #destroy_method
        }
    )
    .into()
}
