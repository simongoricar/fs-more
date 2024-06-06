use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Args, Debug)]
pub(crate) struct GenerateTreeJsonSchemaCommandArguments {
    #[arg(short = 'o', long = "json-schema-output-file-path")]
    pub(crate) output_file_path: PathBuf,

    #[arg(long = "overwrite-existing-file")]
    pub(crate) overwrite_existing_file: Option<bool>,
}

#[derive(Args, Debug)]
pub(crate) struct GenerateTreeSourcesCommandArguments {
    #[arg(short = 'i', long = "tree-schemas-input-directory-path")]
    pub(crate) tree_schemas_directory_path: PathBuf,

    #[arg(short = 'o', long = "test-harness-crate-output-directory-path")]
    pub(crate) test_harness_crate_directory_path: PathBuf,

    #[arg(long = "overwrite-existing-files")]
    pub(crate) overwrite_existing_files: Option<bool>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum CliCommand {
    #[command(name = "generate-tree-json-schema")]
    GenerateTreeJsonSchema(GenerateTreeJsonSchemaCommandArguments),

    #[command(name = "generate-tree-sources")]
    GenerateTreeSources(GenerateTreeSourcesCommandArguments),
}

#[derive(Parser, Debug)]
#[command(version)]
pub(crate) struct CliArguments {
    #[command(subcommand)]
    pub(crate) command: CliCommand,
}
