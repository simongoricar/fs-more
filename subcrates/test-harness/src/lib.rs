pub mod assertable;
pub mod error;
pub mod prelude;

mod case_sensitivity;
mod path_comparison;

pub use assert_matches::{assert_matches, debug_assert_matches};
pub use case_sensitivity::*;
pub use path_comparison::*;


pub mod trees {
    #[path = "../generated_trees/mod.rs"]
    pub mod structures;

    #[path = "../tree_framework/mod.rs"]
    mod framework;

    pub use framework::*;
}
