use std::{
    fs::{self, OpenOptions},
    io::{prelude::Write, BufWriter},
    path::{Path, PathBuf},
};

use clap::Parser;
use cli::{
    CliArguments,
    CliCommand,
    GenerateTreeJsonSchemaCommandArguments,
    GenerateTreeSourcesCommandArguments,
};
use codegen::final_source_file::generate_rust_source_file_for_schema;
use fs_more_test_harness_tree_schema::schema::FileSystemHarnessSchema;
use miette::{miette, Context, IntoDiagnostic, Result};
use schemars::gen::SchemaGenerator;

mod cli;
mod codegen;
mod name_collision;



fn collect_tree_schemas(
    schema_input_directory_path: &Path,
) -> Result<Vec<(PathBuf, FileSystemHarnessSchema)>> {
    if !schema_input_directory_path.exists() {
        return Err(miette!("The provided tree schema directory path does not exist."));
    } else if !schema_input_directory_path.is_dir() {
        return Err(miette!("THe provided tree schema directory path is not a directory."));
    }


    let mut parsed_schemas = Vec::new();


    let directory_iterator = fs::read_dir(schema_input_directory_path)
        .into_diagnostic()
        .wrap_err("Failed to initialize directory iterator.")?;

    for directory_entry in directory_iterator {
        let directory_entry = directory_entry
            .into_diagnostic()
            .wrap_err("Failed to iterate.")?;

        let entry_path = directory_entry.path();
        let entry_type = directory_entry
            .file_type()
            .into_diagnostic()
            .wrap_err("Unable to determine entry file type.")?;

        if !entry_type.is_file() {
            continue;
        }


        let file_extension = entry_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        if file_extension != "json" {
            continue;
        }


        let file_name = entry_path
            .file_name()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        if file_name.starts_with('.') || file_name.starts_with('_') {
            continue;
        }


        let json_data_string = fs::read_to_string(&entry_path)
            .into_diagnostic()
            .wrap_err("Failed to read JSON file.")?;

        let parsed_schema: FileSystemHarnessSchema = serde_json::from_str(&json_data_string)
            .into_diagnostic()
            .wrap_err_with(|| {
                miette!(
                    "Failed to parse JSON data as FileSystemHarnessSchema. File: {}",
                    entry_path.display()
                )
            })?;

        parsed_schemas.push((entry_path, parsed_schema));
    }

    Ok(parsed_schemas)
}



fn get_generated_tree_directory(
    potential_test_harness_crate_directory: &Path,
    create_missing_generated_trees_directory: bool,
) -> Result<PathBuf> {
    let cargo_toml_file_path = potential_test_harness_crate_directory.join("Cargo.toml");
    if !cargo_toml_file_path.exists() || !cargo_toml_file_path.is_file() {
        return Err(miette!(
            "Invalid test harness crate directory path: missing Cargo.toml"
        ));
    }

    let src_directory_path = potential_test_harness_crate_directory.join("src");
    if !src_directory_path.exists() || !src_directory_path.is_dir() {
        return Err(miette!(
            "Invalid test harness crate directory path: missing src directory"
        ));
    }

    let generated_trees_directory_path = src_directory_path.join("trees/generated");
    if !generated_trees_directory_path.exists() {
        if create_missing_generated_trees_directory {
            fs::create_dir(&generated_trees_directory_path)
                .into_diagnostic()
                .wrap_err("Unable to create missing src/trees/generated directory.")?;
        } else {
            return Err(miette!(
                "Invalid test harness crate directory path: missing src/trees/generated path"
            ));
        }
    }

    Ok(generated_trees_directory_path)
}


fn generate_trees(options: GenerateTreeSourcesCommandArguments) -> Result<()> {
    // Assert the provided test harness crate path is valid.
    let generated_trees_directory =
        get_generated_tree_directory(&options.test_harness_crate_directory_path, true)
            .wrap_err("Failed to compute generated_tree directory.")?;

    // Collect and parse tree JSON schemas.
    let tree_schemas = collect_tree_schemas(&options.tree_schemas_directory_path)
        .wrap_err("Failed to collect schemas.")?;


    for (schema_path, schema) in tree_schemas {
        let schema_name = schema.name.clone();
        let output_file_path = generated_trees_directory.join(format!("{}.rs", schema.file_name));

        println!(
            "Generating source file for {} ({}.rs).",
            schema.name, schema.file_name
        );

        generate_rust_source_file_for_schema(
            &schema_path,
            schema,
            &output_file_path,
            options.overwrite_existing_files.unwrap_or(false),
        )
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to generate source file for schema {}.", schema_name))?;
    }

    Ok(())
}



fn generate_and_save_tree_json_schema(
    options: GenerateTreeJsonSchemaCommandArguments,
) -> Result<()> {
    let tree_schema = SchemaGenerator::default().into_root_schema_for::<FileSystemHarnessSchema>();
    let serialized_tree_schema = serde_json::to_string_pretty(&tree_schema).into_diagnostic()?;


    let mut buffered_writer = {
        let file = match options.overwrite_existing_file.unwrap_or(false) {
            true => OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&options.output_file_path)
                .into_diagnostic()?,
            false => OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&options.output_file_path)
                .into_diagnostic()?,
        };

        BufWriter::new(file)
    };


    buffered_writer
        .write_all(serialized_tree_schema.as_bytes())
        .into_diagnostic()?;


    let mut file = buffered_writer.into_inner().into_diagnostic()?;
    file.flush().into_diagnostic()?;


    Ok(())
}


fn main() -> Result<()> {
    let cli_arguments = CliArguments::parse();

    match cli_arguments.command {
        CliCommand::GenerateTreeJsonSchema(schema_generation_args) => {
            generate_and_save_tree_json_schema(schema_generation_args)
        }
        CliCommand::GenerateTreeSources(codegen_args) => generate_trees(codegen_args),
    }
}
