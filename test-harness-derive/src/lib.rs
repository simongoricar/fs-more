use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use quote::{quote, ToTokens};
use syn::{
    parse_quote,
    punctuated::Punctuated,
    ItemStruct,
    MetaNameValue,
    Token,
};

const ASSERTABLE_ROOT_DIRECTORY_TYPE_NAME: &str = "AssertableRootDirectory";
const ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME: &str = "AssertableFilePath";
const ASSERTABLE_DIRECTORY_PATH_STRUCT_TYPE_NAME: &str =
    "AssertableDirectoryPath";


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FieldPathType {
    Root,
    File,
    Directory,
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

    if last_field_type_segment
        .ident
        .eq(ASSERTABLE_ROOT_DIRECTORY_TYPE_NAME)
    {
        Some(FieldPathType::Root)
    } else if last_field_type_segment
        .ident
        .eq(ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME)
    {
        Some(FieldPathType::File)
    } else if last_field_type_segment
        .ident
        .eq(ASSERTABLE_DIRECTORY_PATH_STRUCT_TYPE_NAME)
    {
        Some(FieldPathType::Directory)
    } else {
        None
    }
}


struct RootField {
    field_ident: syn::Ident,
}

struct FileField {
    field_ident: syn::Ident,
    file_path: syn::LitStr,
    file_contents: Option<syn::Expr>,
}

struct DirectoryField {
    field_ident: syn::Ident,
    directory_path: syn::LitStr,
}

enum ParsedField {
    Root(RootField),
    File(FileField),
    Directory(DirectoryField),
}

struct ParsedStruct {
    all_field_idents: Vec<syn::Ident>,

    root_field: RootField,

    file_fields: Vec<FileField>,

    directory_fields: Vec<DirectoryField>,
}



const ROOT_ATTRIBUTE_NAME: &str = "root";
const FILE_ATTRIBUTE_NAME: &str = "file";
const DIRECTORY_ATTRIBUTE_NAME: &str = "directory";


fn parse_struct_field(field: &syn::Field) -> Option<ParsedField> {
    let path_type_inferred_from_field_type =
        infer_field_path_type_from_field_type(field);

    let Some(field_ident) = field.ident.clone() else {
        abort_call_site!(
            "Missing field name."
        );
    };

    for attribute in &field.attrs {
        if !matches!(attribute.style, syn::AttrStyle::Outer) {
            continue;
        }

        match &attribute.meta {
            syn::Meta::Path(attribute_path) => {
                if !attribute_path.is_ident(ROOT_ATTRIBUTE_NAME) {
                    continue;
                }

                let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                    abort_call_site!(
                        "Field {} has the #[{}] attribute, 
                        but isn't of a recognized assertable type: 
                        expected type {}, got {}.",
                        ROOT_ATTRIBUTE_NAME,
                        field_ident.to_string(),
                        ASSERTABLE_ROOT_DIRECTORY_TYPE_NAME,
                        field.ty.to_token_stream(),
                    );
                };

                if *inferred_path_type != FieldPathType::Root {
                    abort_call_site!(
                        "Field {} has the #[{}] attribute,
                        but isn't of the correct assertable type:
                        expected type {}, got {}.",
                        ROOT_ATTRIBUTE_NAME,
                        field_ident.to_string(),
                        ASSERTABLE_ROOT_DIRECTORY_TYPE_NAME,
                        field.ty.to_token_stream(),
                    );
                }

                return Some(ParsedField::Root(RootField { field_ident }));
            }
            syn::Meta::List(list_attribute) => {
                if list_attribute.path.is_ident(FILE_ATTRIBUTE_NAME) {
                    let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                        abort_call_site!(
                            "Field {} has the #[{}(...)] attribute,
                            but isn't of a recognized assertable type:
                            expected type {}, got {}.",
                            FILE_ATTRIBUTE_NAME,
                            field_ident.to_string(),
                            ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME,
                            field.ty.to_token_stream(),
                        );
                    };

                    if *inferred_path_type != FieldPathType::File {
                        abort_call_site!(
                            "Field {} has the #[{}(...)] attribute,
                            but isn't of the correct assertable type:
                            expected type {}, got {}.",
                            FILE_ATTRIBUTE_NAME,
                            field_ident.to_string(),
                            ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME,
                            field.ty.to_token_stream(),
                        );
                    };

                    let subattributes: Punctuated<MetaNameValue, Token![,]> = list_attribute
                        .parse_args_with(Punctuated::parse_terminated)
                        .unwrap_or_else(|_| abort_call_site!(
                            "Expected a #[{}({} = \"string literal\", {} = expression resolving to Vec of u8)] 
                            attribute, got {} instead.",
                            FILE_ATTRIBUTE_NAME,
                            FILE_PATH_SUBATTRIBUTE_NAME,
                            FILE_CONTENT_SUBATTRIBUTE_NAME,
                            list_attribute.to_token_stream()
                        ));

                    let mut path_subattribute: Option<syn::LitStr> = None;
                    let mut contents_subattribute: Option<syn::Expr> = None;

                    const FILE_PATH_SUBATTRIBUTE_NAME: &str = "path";
                    const FILE_CONTENT_SUBATTRIBUTE_NAME: &str = "content";

                    for subattribute in subattributes {
                        if subattribute
                            .path
                            .is_ident(FILE_PATH_SUBATTRIBUTE_NAME)
                        {
                            let syn::Expr::Lit(path_literal) = &subattribute.value else {
                                abort_call_site!(
                                    "Expected #[{}(..., {} = \"string literal\")], got {}.",
                                    FILE_ATTRIBUTE_NAME,
                                    FILE_PATH_SUBATTRIBUTE_NAME,
                                    subattribute.to_token_stream(),
                                );
                            };

                            let syn::Lit::Str(path_str_literal) = &path_literal.lit else {
                                abort_call_site!(
                                    "Expected #[{}(..., {} = \"string literal\")], got {}.",
                                    FILE_ATTRIBUTE_NAME,
                                    FILE_PATH_SUBATTRIBUTE_NAME,
                                    subattribute.to_token_stream(),
                                );
                            };

                            if path_subattribute.is_some() {
                                abort_call_site!(
                                    "Unexpected contents of #[{}(...)] attribute: 
                                    field \"{}\" appears more than once.",
                                    FILE_ATTRIBUTE_NAME,
                                    FILE_PATH_SUBATTRIBUTE_NAME
                                );
                            }

                            path_subattribute = Some(path_str_literal.clone());
                        } else if subattribute
                            .path
                            .is_ident(FILE_CONTENT_SUBATTRIBUTE_NAME)
                        {
                            if contents_subattribute.is_some() {
                                abort_call_site!(
                                    "Unexpected contents of #[{}(...)] attribute: 
                                    field \"{}\" appears more than once.",
                                    FILE_ATTRIBUTE_NAME,
                                    FILE_CONTENT_SUBATTRIBUTE_NAME
                                );
                            }

                            contents_subattribute = Some(subattribute.value);
                        } else {
                            abort_call_site!(
                                "Unexpected contents of #[{}(...)] attribute:
                                expected fields {} and/or {}, got \"{}\" instead.",
                                FILE_ATTRIBUTE_NAME,
                                FILE_PATH_SUBATTRIBUTE_NAME,
                                FILE_CONTENT_SUBATTRIBUTE_NAME,
                                subattribute.to_token_stream()
                            );
                        }
                    }

                    let Some(path_subattribute) = path_subattribute else {
                        abort_call_site!(
                            "Attribute #[{}(...)] is missing field: \'{} = \"string literal\"\'",
                            FILE_ATTRIBUTE_NAME,
                            FILE_PATH_SUBATTRIBUTE_NAME,
                        );
                    };

                    return Some(ParsedField::File(FileField {
                        field_ident,
                        file_path: path_subattribute,
                        file_contents: contents_subattribute,
                    }));
                } else if list_attribute.path.is_ident(DIRECTORY_ATTRIBUTE_NAME)
                {
                    let Some(inferred_path_type) = path_type_inferred_from_field_type.as_ref() else {
                        abort_call_site!(
                            "Field {} has the #[{}(...)] attribute,
                            but isn't of a recognized assertable type:
                            expected type {}, got {}.",
                            DIRECTORY_ATTRIBUTE_NAME,
                            field_ident.to_string(),
                            ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME,
                            field.ty.to_token_stream(),
                        );
                    };

                    if *inferred_path_type != FieldPathType::Directory {
                        abort_call_site!(
                            "Field {} has the #[{}(...)] attribute,
                            but isn't of the correct assertable type:
                            expected type {}, got {}.",
                            DIRECTORY_ATTRIBUTE_NAME,
                            field_ident.to_string(),
                            ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME,
                            field.ty.to_token_stream(),
                        );
                    };

                    let subattributes: Punctuated<MetaNameValue, Token![,]> =
                        list_attribute
                            .parse_args_with(Punctuated::parse_terminated)
                            .unwrap_or_else(|_| {
                                abort_call_site!(
                                    "Expected a #[{}({} = \"string literal\")] 
                            attribute, got {} instead.",
                                    DIRECTORY_ATTRIBUTE_NAME,
                                    DIRECTORY_PATH_SUBATTRIBUTE_NAME,
                                    list_attribute.to_token_stream()
                                )
                            });


                    let mut path_subattribute: Option<syn::LitStr> = None;

                    const DIRECTORY_PATH_SUBATTRIBUTE_NAME: &str = "path";

                    for subattribute in subattributes {
                        if !subattribute
                            .path
                            .is_ident(DIRECTORY_PATH_SUBATTRIBUTE_NAME)
                        {
                            continue;
                        }


                        let syn::Expr::Lit(path_literal) = &subattribute.value else {
                            abort_call_site!(
                                "Expected #[{}(..., {} = \"string literal\")], got {}.",
                                DIRECTORY_ATTRIBUTE_NAME,
                                DIRECTORY_PATH_SUBATTRIBUTE_NAME,
                                subattribute.to_token_stream(),
                            );
                        };

                        let syn::Lit::Str(path_str_literal) = &path_literal.lit else {
                            abort_call_site!(
                                "Expected #[{}(..., {} = \"string literal\")], got {}.",
                                DIRECTORY_ATTRIBUTE_NAME,
                                DIRECTORY_PATH_SUBATTRIBUTE_NAME,
                                subattribute.to_token_stream(),
                            );
                        };

                        if path_subattribute.is_some() {
                            abort_call_site!(
                                "Unexpected contents of #[{}(...)] attribute: 
                                field \"{}\" appears more than once.",
                                DIRECTORY_ATTRIBUTE_NAME,
                                DIRECTORY_PATH_SUBATTRIBUTE_NAME,
                            );
                        }

                        path_subattribute = Some(path_str_literal.clone());
                    }

                    let Some(path_subattribute) = path_subattribute else {
                        abort_call_site!(
                            "Attribute #[{}(...)] is missing field: \'{} = \"string literal\"\'",
                            DIRECTORY_ATTRIBUTE_NAME,
                            DIRECTORY_PATH_SUBATTRIBUTE_NAME,
                        );
                    };

                    return Some(ParsedField::Directory(DirectoryField {
                        field_ident,
                        directory_path: path_subattribute,
                    }));
                }
            }
            _ => {}
        }
    }

    // If we haven't returned at this point, this means the field could just be
    // some random field that isn't annotated/important for this macro.
    // However, if we were able to find a speficic assertable type on it, we should abort
    // as those types should only really be used along with the macro and reaching this
    // point in the code might indicate the user forgot to add
    // a #[root]/#[file(...)]/#[directory(...)] field attribute.
    match path_type_inferred_from_field_type {
        Some(_) => {
            abort_call_site!(
                "Field {} is of a recognized assertable type {},
                but is missing the associated #[root]/#[file(...)]/#[directory(...)]
                attribute. Did you forget to add a field attribute?",
                field_ident.to_string(),
                field.ty.to_token_stream(),
            );
        }
        None => None,
    }
}

fn remove_our_macro_attributes_from_field(field: &mut syn::Field) {
    field.attrs.retain(|attribute| {
        if !matches!(attribute.style, syn::AttrStyle::Outer) {
            return true;
        }

        match &attribute.meta {
            syn::Meta::Path(path_attribute) => {
                !path_attribute.is_ident(ROOT_ATTRIBUTE_NAME)
            }
            syn::Meta::List(list_attribute) => {
                !(list_attribute.path.is_ident(FILE_ATTRIBUTE_NAME)
                    || list_attribute.path.is_ident(DIRECTORY_ATTRIBUTE_NAME))
            }
            _ => true,
        }
    });
}

fn add_documentation_to_field(
    field: &mut syn::Field,
    parsed_data: &ParsedField,
) {
    let documentation_lines = match parsed_data {
        ParsedField::Root(root_field) => {
            let root_field_ident = &root_field.field_ident;

            vec![
                format!(" Root directory."),
                format!(""),
                format!(
                    " ##### Autogenerated by the `#[fs_harness_tree]` procedural macro \
                    (for root field `{root_field_ident}`)."
                ),
            ]
        }
        ParsedField::File(file_field) => {
            let file_field_ident = &file_field.field_ident;
            let file_path = &file_field.file_path;

            let file_path_string = file_path.to_token_stream().to_string();
            let file_path_str_stripped_quotes = file_path_string
                .strip_prefix('"')
                .unwrap()
                .strip_suffix('"')
                .unwrap();

            vec![
                format!(" File path: `{file_path_str_stripped_quotes}`."),
                format!(""),
                format!(
                    " ##### Autogenerated by the `#[fs_harness_tree]` procedural macro \
                    (for file field `{file_field_ident}`)."
                ),
            ]
        }
        ParsedField::Directory(directory_field) => {
            let directory_field_ident = &directory_field.field_ident;
            let directory_path = &directory_field.directory_path;

            let directory_path_string =
                directory_path.to_token_stream().to_string();
            let directoryr_path_str_stripped_quotes = directory_path_string
                .strip_prefix('"')
                .unwrap()
                .strip_suffix('"')
                .unwrap();

            vec![
                format!(" Directory path: `{directoryr_path_str_stripped_quotes}`."),
                format!(""),
                format!(
                    " ##### Autogenerated by the `#[fs_harness_tree]` procedural macro \
                    (for directory field `{directory_field_ident}`)."
                ),
            ]
        }
    };

    field
        .attrs
        .extend(documentation_lines.into_iter().map(|line| {
            parse_quote! {
                #[doc = #line]
            }
        }));
}

fn parse_struct_data(mut struct_data: ItemStruct) -> (ItemStruct, ParsedStruct) {
    let mut all_field_idents: Vec<syn::Ident> =
        Vec::with_capacity(struct_data.fields.len());

    let mut root_field: Option<RootField> = None;
    let mut directory_fields: Vec<DirectoryField> = Vec::new();
    let mut file_fields: Vec<FileField> = Vec::new();

    let syn::Fields::Named(named_fields) = &mut struct_data.fields else {
        abort_call_site!(
            "Can only be used on structs with named fields."
        );
    };

    for field in &mut named_fields.named {
        let Some(parsed_field) = parse_struct_field(field) else {
            continue;
        };

        remove_our_macro_attributes_from_field(field);
        add_documentation_to_field(field, &parsed_field);

        match parsed_field {
            ParsedField::Root(new_root_field) => {
                if root_field.is_some() {
                    abort_call_site!(
                        "Found more than one #[root]-annotated struct field (only one is allowed)!"
                    );
                };

                all_field_idents.push(new_root_field.field_ident.clone());
                root_field = Some(new_root_field);
            }
            ParsedField::File(new_file_field) => {
                all_field_idents.push(new_file_field.field_ident.clone());
                file_fields.push(new_file_field);
            }
            ParsedField::Directory(new_directory_field) => {
                all_field_idents.push(new_directory_field.field_ident.clone());
                directory_fields.push(new_directory_field);
            }
        }
    }

    let Some(root_field) = root_field else {
        abort_call_site!(
            "The struct is missing a #[root]-annotated field (precisely one is required)!"
        );
    };

    (
        struct_data,
        ParsedStruct {
            all_field_idents,
            root_field,
            file_fields,
            directory_fields,
        },
    )
}

fn generate_initialization_expression_for_root_field(
    field: &RootField,
    temporary_dir_variable_ident: syn::Ident,
) -> proc_macro2::TokenStream {
    let assertable_root_directory_type_ident: syn::Ident = syn::Ident::new(
        ASSERTABLE_ROOT_DIRECTORY_TYPE_NAME,
        Span::call_site(),
    );

    let field_ident = &field.field_ident;

    quote! {
        let #field_ident = #assertable_root_directory_type_ident::new(#temporary_dir_variable_ident);
    }
}

fn generate_initialization_expression_for_file_field(
    field: &FileField,
    temporary_dir_variable_ident: syn::Ident,
) -> proc_macro2::TokenStream {
    let assertable_file_path_type_ident: syn::Ident = syn::Ident::new(
        ASSERTABLE_FILE_PATH_STRUCT_TYPE_NAME,
        Span::call_site(),
    );

    let field_ident = &field.field_ident;
    let file_path = &field.file_path;

    if let Some(file_contents) = &field.file_contents {
        quote! {
            let #field_ident = {
                let child_path = #temporary_dir_variable_ident.child(#file_path);
                let original_contents = &#file_contents;

                child_path.write_binary(original_contents)?;

                assert_eq!(
                    &std::fs::read(child_path.path()).unwrap(),
                    original_contents
                );

                #assertable_file_path_type_ident::from_child_path(child_path, original_contents)
            };
        }
    } else {
        quote! {
            let #field_ident = {
                let child_path = #temporary_dir_variable_ident.child(#file_path);

                #assertable_file_path_type_ident::from_child_path(child_path, Vec::with_capacity(0))
            };
        }
    }
}

fn generate_initialization_expression_for_directory_field(
    field: &DirectoryField,
    temporary_dir_variable_ident: syn::Ident,
) -> proc_macro2::TokenStream {
    let assertable_directory_path_type_ident: syn::Ident = syn::Ident::new(
        ASSERTABLE_DIRECTORY_PATH_STRUCT_TYPE_NAME,
        Span::call_site(),
    );

    let field_ident = &field.field_ident;
    let directory_path = &field.directory_path;

    quote! {
        let #field_ident = {
            let child_path = #temporary_dir_variable_ident.child(#directory_path);
            child_path.create_dir_all()?;

            #assertable_directory_path_type_ident::from_child_path(child_path)
        };
    }
}

fn generate_impl(
    parsed: ParsedStruct,
    struct_name: syn::Ident,
    struct_impl_generics: syn::ImplGenerics,
    struct_ty_generics: syn::TypeGenerics,
    struct_where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let initialization_method: proc_macro2::TokenStream = {
        let temporary_dir_variable_ident =
            syn::Ident::new("temporary_directory", Span::call_site());

        let root_field_initialization_expr =
            generate_initialization_expression_for_root_field(
                &parsed.root_field,
                temporary_dir_variable_ident.clone(),
            );

        let directory_fields_initialization_exprs = parsed
            .directory_fields
            .iter()
            .map(|field| {
                generate_initialization_expression_for_directory_field(
                    field,
                    temporary_dir_variable_ident.clone(),
                )
            })
            .collect::<Vec<_>>();

        let file_fields_initialization_exprs = parsed
            .file_fields
            .iter()
            .map(|field| {
                generate_initialization_expression_for_file_field(
                    field,
                    temporary_dir_variable_ident.clone(),
                )
            })
            .collect::<Vec<_>>();


        let all_field_idents = parsed.all_field_idents;

        quote! {
            pub fn new() -> std::result::Result<Self, assert_fs::fixture::FixtureError> {
                use assert_fs::fixture::PathChild;
                use assert_fs::fixture::FileWriteBin;
                use assert_fs::fixture::PathCreateDir;

                let #temporary_dir_variable_ident = assert_fs::TempDir::new()?;

                #(#directory_fields_initialization_exprs)*
                #(#file_fields_initialization_exprs)*
                #root_field_initialization_expr

                Ok(Self {
                    #(#all_field_idents),*
                })

            }
        }
    };

    let teardown_method: proc_macro2::TokenStream = {
        let root_field_ident = parsed.root_field.field_ident;

        quote! {
            pub fn destroy(self) -> std::result::Result<(), assert_fs::fixture::FixtureError> {
                let temporary_directory = self.#root_field_ident.into_temp_dir();
                temporary_directory.close()?;

                Ok(())
            }
        }
    };

    quote!(
        impl #struct_impl_generics #struct_name #struct_ty_generics #struct_where_clause {
            #initialization_method
            #teardown_method
        }
    )
}

#[proc_macro_attribute]
pub fn fs_harness_tree(
    _attributes: TokenStream,
    data: TokenStream,
) -> TokenStream {
    let struct_data = match syn::parse::<ItemStruct>(data) {
        Ok(data) => data,
        Err(_) => {
            abort_call_site!(
                "Can't parse input (can only be used on structs with named fields)."
            );
        }
    };

    let (modified_struct, parsed_data) = parse_struct_data(struct_data);


    let struct_name = modified_struct.ident.clone();
    let (impl_generics, ty_generics, where_clause) =
        modified_struct.generics.split_for_impl();

    let new_impl = generate_impl(
        parsed_data,
        struct_name,
        impl_generics,
        ty_generics,
        where_clause,
    );

    // TODO Parse struct fields, remove any parsed field attributes, add field documentation.
    // TODO Then, do the same as before in the derive macro.

    quote! {
        #modified_struct
        #new_impl
    }
    .into()
}
