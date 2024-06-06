#[deprecated]
pub mod assertable_old;

pub mod assertable;
mod content_assertion;
pub mod error;

mod seeded_data;

#[path = "generated_trees/mod.rs"]
pub mod trees;

#[deprecated]
pub mod trees_old;

pub mod tree_framework;

pub fn is_temporary_directory_case_sensitive() -> bool {
    let temporary_dir = tempfile::tempdir().unwrap();

    let uppercase_file_path = temporary_dir.path().join("README.txt");
    let lowercase_file_path = temporary_dir.path().join("readme.txt");

    std::fs::File::create_new(uppercase_file_path).unwrap();

    let is_case_sensitive = std::fs::File::create_new(lowercase_file_path).is_ok();


    temporary_dir.close().unwrap();

    is_case_sensitive
}
