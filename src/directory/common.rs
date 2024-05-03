use std::path::{Path, PathBuf};

use crate::{error::SourceSubPathNotUnderBaseSourceDirectory, file::ExistingFileBehaviour};


/// Rules that dictate how existing destination sub-directories
/// are handled when copying or moving.
///
/// See also: [`DestinationDirectoryRule`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ExistingSubDirectoryBehaviour {
    /// An existing destination sub-directory
    /// will cause an error if the copy operation requires
    /// copying into it.
    Abort,

    /// An existing destination sub-directory will have no effect.
    Continue,
}

/// Specifies whether you allow the destination directory to exist
/// before copying or moving files or directories into it.
///
/// If you allow the target directory to exist, you can also specify whether it must be empty;
/// if not, you may also specify whether you allow files and directories to be overwritten.
///
///
/// ## Defaults
/// [`Default`] is implemented for this enum. The default value is [`DestinationDirectoryRule::AllowEmpty`].
///
///
/// ## Examples
/// If you want the associated directory copying or moving function to
/// *return an error if the target directory already exists*, use [`DestinationDirectoryRule::DisallowExisting`];
///
/// If you want to copy into an *existing empty target directory*, you should use [`DestinationDirectoryRule::AllowEmpty`]
/// (this rule *does not require* the target directory to exist and will create one if missing).
///
/// If the target directory could already exist and have some files or directories in it, you can use the following rule:
///
/// ```
/// # use fs_more::directory::DestinationDirectoryRule;
/// let rules = DestinationDirectoryRule::AllowNonEmpty {
///     create_missing_subdirectories: false,
///     overwrite_existing_files: false,
/// };
/// ```
///
/// This will still not overwrite any overlapping files (i.e. a merge without overwrites will be performed).
///
/// If you want files and/or directories to be overwritten, you may set the flags for overwriting to `true`:
///
/// ```
/// # use fs_more::directory::DestinationDirectoryRule;
/// let rules = DestinationDirectoryRule::AllowNonEmpty {
///     create_missing_subdirectories: true,
///     overwrite_existing_files: true,
/// };
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DestinationDirectoryRule {
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
    /// like a merge that will not touch existing destination files.
    /// If they are `true`, it behaves like a merge that will
    /// overwrite any existing files and create any missing directories.
    AllowNonEmpty {
        /// How to behave for destination files that already exist.
        existing_destination_file_behaviour: ExistingFileBehaviour,

        /// How to behave for destination sub-directories that already exist.
        existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour,
    },
}

impl Default for DestinationDirectoryRule {
    /// The default value for this struct is [`Self::AllowEmpty`].
    fn default() -> Self {
        Self::AllowEmpty
    }
}

impl DestinationDirectoryRule {
    pub(crate) fn allows_overwriting_existing_destination_files(&self) -> bool {
        matches!(
            self,
            Self::AllowNonEmpty {
                existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
                ..
            }
        )
    }

    pub(crate) fn ignores_existing_destination_sub_directories(&self) -> bool {
        matches!(
            self,
            Self::AllowNonEmpty {
                existing_destination_subdirectory_behaviour:
                    ExistingSubDirectoryBehaviour::Continue,
                ..
            }
        )
    }
}



/// Applies the same sub-path that `source_sub_path` has, relative to `source_base_directory_path`,
/// onto `target_base_directory_path`.
///
/// `source_base_directory_path` is the base source directory path,
/// and `source_sub_path` *must* be a descendant of that path.
/// `target_base_directory_path` can be an arbitrary target directory path.
///
/// Returns a [`DirectoryError::SourceSubPathEscapesSourceDirectory`]
/// if the `source_sub_path` is not a sub-path of `source_base_directory_path`.
///
/// # Example
/// ```ignore
/// # use std::path::Path;
/// # use fs_more::directory::copy::join_relative_source_path_onto_destination;
///
/// let foo = Path::new("/foo");
/// let foo_hello_world = Path::new("/foo/abc/hello-world.txt");
/// let bar = Path::new("/bar");
///
/// assert_eq!(
///     join_relative_source_path_onto_destination(
///         foo,
///         foo_hello_world,
///         bar
///     ).unwrap(),
///     Path::new("/bar/abc/hello-world.txt")
/// );
/// ```
pub(crate) fn join_relative_source_path_onto_destination(
    source_base_directory_path: &Path,
    source_sub_path: &Path,
    target_base_directory_path: &Path,
) -> Result<PathBuf, SourceSubPathNotUnderBaseSourceDirectory> {
    // Strip the base source directory path from the full source path
    // and place it on top of the target base directory path.

    if source_base_directory_path.eq(source_sub_path) {
        return Ok(target_base_directory_path.to_path_buf());
    }

    let source_sub_path_relative_to_base = source_sub_path
        .strip_prefix(source_base_directory_path)
        .map_err(|_| SourceSubPathNotUnderBaseSourceDirectory {
            path: source_base_directory_path.join(source_sub_path),
        })?;

    Ok(target_base_directory_path.join(source_sub_path_relative_to_base))
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
            join_relative_source_path_onto_destination(root_a, foo, root_b).unwrap(),
            Path::new("/different/root/some/content"),
            "rejoin_source_subpath_onto_target did not rejoin the path properly."
        );

        let foo = Path::new("/foo");
        let foo_hello_world = Path::new("/foo/abc/hello-world.txt");
        let bar = Path::new("/bar");

        assert_eq!(
            join_relative_source_path_onto_destination(foo, foo_hello_world, bar).unwrap(),
            Path::new("/bar/abc/hello-world.txt")
        );
    }

    #[test]
    fn error_on_subpath_not_being_under_source_root() {
        let root_a = Path::new("/hello/there");
        let foo = Path::new("/completely/different/path");
        let root_b = Path::new("/different/root");

        let rejoin_result = join_relative_source_path_onto_destination(root_a, foo, root_b);

        assert!(
            rejoin_result.is_err(),
            "rejoin_source_subpath_onto_target did not return Err when \
            the source path to rejoin wasn't under the source root path"
        );
    }
}
