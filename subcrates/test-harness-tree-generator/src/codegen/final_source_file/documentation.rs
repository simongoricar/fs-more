use std::collections::VecDeque;

use fs_more_test_harness_tree_schema::schema::{
    FileDataConfiguration,
    FileSystemHarnessEntry,
    FileSystemHarnessSchema,
};
use itertools::Itertools;

use crate::codegen::AnyGeneratedEntry;



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
            formatted_line.push_str(
                if matches!(next_item.entry, FileSystemHarnessEntry::Directory(_)) {
                    "|-- "
                } else {
                    "|-> "
                },
            );
        }


        formatted_line.push_str(
            match next_item.entry {
                FileSystemHarnessEntry::File(file_entry) => {
                    let short_file_description = match file_entry
                        .data
                        .as_ref()
                        .unwrap_or(&FileDataConfiguration::Empty)
                    {
                        FileDataConfiguration::Empty => "empty".to_string(),
                        FileDataConfiguration::Text { content } => {
                            let formatted_length_of_content =
                                humansize::format_size(content.len(), humansize::BINARY);

                            format!("text data, {}", formatted_length_of_content)
                        }
                        FileDataConfiguration::DeterministicRandom {
                            file_size_bytes, ..
                        } => {
                            let formatted_number_of_bytes =
                                humansize::format_size(*file_size_bytes, humansize::BINARY);

                            format!("binary data, {}", formatted_number_of_bytes)
                        }
                    };

                    let entry_suffix_if_contains_id = match file_entry.id.as_ref() {
                        Some(entry_id) => {
                            format!(" [ID \"{}\"]", entry_id)
                        }
                        None => "".to_string(),
                    };


                    format!(
                        "{} ({}){}",
                        file_entry.name, short_file_description, entry_suffix_if_contains_id
                    )
                }
                FileSystemHarnessEntry::Directory(dir_entry) => {
                    let entry_suffix_if_contains_id = match dir_entry.id.as_ref() {
                        Some(entry_id) => {
                            format!(" [ID \"{}\"]", entry_id)
                        }
                        None => "".to_string(),
                    };

                    format!("{}{}", dir_entry.name, entry_suffix_if_contains_id)
                }
                FileSystemHarnessEntry::Symlink(symlink_entry) => {
                    let entry_suffix_if_contains_id = match symlink_entry.id.as_ref() {
                        Some(entry_id) => {
                            format!(" [ID \"{}\"]", entry_id)
                        }
                        None => "".to_string(),
                    };

                    format!(
                        "{} (symlink to \"{}\"){}",
                        symlink_entry.name,
                        symlink_entry.destination_entry_id,
                        entry_suffix_if_contains_id
                    )
                }
                FileSystemHarnessEntry::BrokenSymlink(broken_symlink_entry) => {
                    let entry_suffix_if_contains_id = match broken_symlink_entry.id.as_ref() {
                        Some(entry_id) => {
                            format!(" [ID \"{}\"]", entry_id)
                        }
                        None => "".to_string(),
                    };

                    format!(
                        "{} (broken symlink to \"{}\"){}",
                        broken_symlink_entry.name,
                        broken_symlink_entry.destination_relative_path,
                        entry_suffix_if_contains_id
                    )
                }
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
#![allow(dead_code)]
#![allow(unused)]\
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
