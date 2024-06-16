pub mod assertable;
pub mod error;

#[path = "generated_trees/mod.rs"]
pub mod trees;

mod case_sensitivity;
pub use case_sensitivity::*;
mod path_comparison;
pub use assert_matches::{assert_matches, debug_assert_matches};
pub use path_comparison::*;


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
