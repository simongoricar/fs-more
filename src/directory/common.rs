use std::path::{Path, PathBuf};

use crate::error::DirectoryError;

/// Specifies whether you allow the target directory to exist
/// before copying or moving files or directories into it.
///
/// If you allow the target directory to exist, you can also specify whether it must be empty;
/// if not, you may also specify whether you allow files and directories to be overwritten.
///
/// ## Defaults
/// [`Default`] is implemented for this enum. The default value is [`TargetDirectoryRule::AllowEmpty`].
///
/// ## Examples
/// If you want the associated directory copying or moving function to
/// *return an error if the target directory already exists*, use [`TargetDirectoryRule::DisallowExisting`];
///
/// If you want to copy into an *existing empty target directory*, you should use [`TargetDirectoryRule::AllowEmpty`]
/// (this rule *does not require* the target directory to exist and will create one if missing).
///
/// If the target directory could already exist and have some files or directories in it, you can use the following rule:
/// ```rust
/// # use fs_more::directory::TargetDirectoryRule;
/// let rules = TargetDirectoryRule::AllowNonEmpty {
///     overwrite_existing_subdirectories: false,
///     overwrite_existing_files: false,
/// };
/// ```
///
/// This will still not overwrite any overlapping files (i.e. a merge without overwrites will be performed).
///
/// If you want files and/or directories to be overwritten, you may set the flags for overwriting to `true`:
/// ```rust
/// # use fs_more::directory::TargetDirectoryRule;
/// let rules = TargetDirectoryRule::AllowNonEmpty {
///     overwrite_existing_subdirectories: true,
///     overwrite_existing_files: true,
/// };
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetDirectoryRule {
    /// Indicates the associated function should return an error if the target directory already exists.
    DisallowExisting,

    /// Indicates the associated function should return an error if the target directory
    /// exists *and is not empty*.
    AllowEmpty,

    /// Indicates that an existing non-empty target directory should not cause an error.
    /// **Do not use this as a default if you're not sure what rule to choose.**
    ///
    /// This can, if the target directory already has some content, end up in a target directory
    /// with *merged* source and target directory contents. Unless you know you want this,
    /// you probably want to avoid this option.
    ///
    /// If the `overwrite_*` options are `false`, this essentially behaves
    /// like a merge that will not touch existing target files.
    /// If they are `true`, it behaves like a merge that will
    /// overwrite any existing files and create any missing directories.
    AllowNonEmpty {
        /// If enabled, the associated function will return
        /// `Err(`[`DirectoryError::TargetItemAlreadyExists`][crate::error::DirectoryError::TargetItemAlreadyExists]`)`
        /// if a target directory or any of subdirectories that would otherwise need to be freshly created already exist.
        overwrite_existing_subdirectories: bool,

        /// If enabled, the associated function will return
        /// `Err(`[`DirectoryError::TargetItemAlreadyExists`][crate::error::DirectoryError::TargetItemAlreadyExists]`)`
        /// if a target file we would otherwise freshly create and copy into already exists.
        overwrite_existing_files: bool,
    },
}

impl Default for TargetDirectoryRule {
    fn default() -> Self {
        Self::AllowEmpty
    }
}

impl TargetDirectoryRule {
    /// Indicates whether this rule allows the target directory
    /// to exist before performing an operation.
    pub fn allows_existing_target_directory(&self) -> bool {
        !matches!(self, Self::DisallowExisting)
    }

    /// Indicates whether this rule allows existing files
    /// in the target directory to be overwritten with contents of the source.
    pub fn should_overwrite_existing_files(&self) -> bool {
        match self {
            TargetDirectoryRule::DisallowExisting => false,
            TargetDirectoryRule::AllowEmpty => false,
            TargetDirectoryRule::AllowNonEmpty {
                overwrite_existing_files,
                ..
            } => *overwrite_existing_files,
        }
    }

    /// Indicates whether this rule allows existing (sub)directories
    /// in the target directory to be "overwritten" with contents of the source (sub)directory.
    pub fn should_overwrite_existing_directories(&self) -> bool {
        match self {
            TargetDirectoryRule::DisallowExisting => false,
            TargetDirectoryRule::AllowEmpty => false,
            TargetDirectoryRule::AllowNonEmpty {
                overwrite_existing_subdirectories,
                ..
            } => *overwrite_existing_subdirectories,
        }
    }
}



/// Given a source root path, a target root path and the source path to rejoin,
/// this function takes the `source_path_to_rejoin`, removes the prefix provided by `source_root_path`
/// and repplies that relative path back onto the `target_root_path`.
///
/// Returns a [`DirectoryError::SubdirectoryEscapesRoot`] if the `source_path_to_rejoin`
/// is not a subpath of `source_root_path`.
///
/// ## Example
/// ```ignore
/// # use std::path::Path;
/// # use fs_more::directory::copy::rejoin_source_subpath_onto_target;
///
/// let root_a = Path::new("/hello/there");
/// let foo = Path::new("/hello/there/some/content");
/// let root_b = Path::new("/different/root");
///
/// assert_eq!(
///     rejoin_source_subpath_onto_target(
///         root_a,
///         foo,
///         root_b
///     ).unwrap(),
///     Path::new("/different/root/some/content")
/// );
/// ```
pub(crate) fn rejoin_source_subpath_onto_target(
    source_root_path: &Path,
    source_path_to_rejoin: &Path,
    target_root_path: &Path,
) -> Result<PathBuf, DirectoryError> {
    // Strip the source subdirectory path from the full source path
    // and place it on top of the target directory path.
    let source_relative_subdirectory_path = if source_root_path.eq(source_path_to_rejoin) {
        Path::new("")
    } else {
        source_path_to_rejoin
            .strip_prefix(source_root_path)
            .map_err(|_| DirectoryError::OtherReason {
                reason: String::from("provided source path escapes its source root"),
            })?
    };

    Ok(target_root_path.join(source_relative_subdirectory_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properly_rejoin_source_subpath_onto_target() {
        let root_a = Path::new("/hello/there");
        let foo = Path::new("/hello/there/some/content");
        let root_b = Path::new("/different/root");

        assert_eq!(
            rejoin_source_subpath_onto_target(root_a, foo, root_b).unwrap(),
            Path::new("/different/root/some/content"),
            "rejoin_source_subpath_onto_target did not rejoin the path properly."
        );
    }

    #[test]
    fn error_on_subpath_not_being_under_source_root() {
        let root_a = Path::new("/hello/there");
        let foo = Path::new("/completely/different/path");
        let root_b = Path::new("/different/root");

        let rejoin_result = rejoin_source_subpath_onto_target(root_a, foo, root_b);

        assert!(
            rejoin_result.is_err(),
            "rejoin_source_subpath_onto_target did not return Err when \
            the source path to rejoin wasn't under the source root path"
        );

        let rejoin_err = rejoin_result.unwrap_err();

        match rejoin_err {
            DirectoryError::OtherReason { reason } => {
                if reason != "provided source path escapes its source root" {
                    panic!(
                        "rejoin_source_subpath_onto_target returned DirectoryError::OtherReason \
                        with the following reason: {}",
                        reason
                    );
                }
            }
            _ => panic!("Unexpected error: {}", rejoin_err),
        }
    }
}
