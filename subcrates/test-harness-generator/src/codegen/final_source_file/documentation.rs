use std::collections::VecDeque;

use itertools::Itertools;

use crate::{
    codegen::AnyGeneratedEntry,
    schema::{FileDataConfiguration, FileSystemHarnessEntry, FileSystemHarnessSchema},
};



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



pub(super) struct DocumentationPieces {
    pub(super) module_documentation: String,
    pub(super) tree_root_struct_documentation: String,
}


pub(super) fn construct_documentation(
    schema: &FileSystemHarnessSchema,
    generated_entries: &[AnyGeneratedEntry],
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



    let formatted_struct_field_list = generated_entries
        .iter()
        .map(|entry| {
            let field_name_ident = entry.actual_field_name_ident_on_parent();
            let field_type_ident = entry.struct_type_ident();

            format!("- `{}` (see [`{}`])", field_name_ident, field_type_ident)
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
