use std::path::{Path, PathBuf};

use crate::{error::SourceSubPathNotUnderBaseSourceDirectory, file::ExistingFileBehaviour};


/// Rules that dictate how existing destination sub-directories
/// are handled when copying or moving.
///
/// See also: [`DestinationDirectoryRule`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ExistingSubDirectoryBehaviour {
    /// An existing destination sub-directory will cause an error.
    Abort,

    /// An existing destination sub-directory will have no effect.
    Continue,
}


/// Specifies whether you allow the destination directory to exist
/// before copying or moving files or directories into it.
///
/// If you allow the destination directory to exist, you can also specify whether it must be empty;
/// if not, you may also specify how to behave for existing destination files and directories.
///
///
/// # Defaults
/// [`Default`] is implemented for this enum. The default value is [`DestinationDirectoryRule::AllowEmpty`].
///
///
/// # Examples
/// If you want the associated directory copying or moving function to
/// return an error if the base destination directory already exists,
/// use [`DestinationDirectoryRule::DisallowExisting`].
///
/// If you want to copy into an existing—but empty—destination directory, use [`DestinationDirectoryRule::AllowEmpty`].
/// This rule does not require the destination directory to exist, only allows it.
///
/// <br>
///
/// If the destination directory is allowed to exist *and* contain existing files or sub-directories,
/// but you don't want to overwrite any of the existing files, you can use the following rule:
///
/// ```no_run
/// # use fs_more::directory::DestinationDirectoryRule;
/// # use fs_more::directory::ExistingSubDirectoryBehaviour;
/// # use fs_more::file::ExistingFileBehaviour;
/// let rules = DestinationDirectoryRule::AllowNonEmpty {
///     existing_destination_file_behaviour: ExistingFileBehaviour::Abort,
///     existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Continue,
/// };
/// ```
///
/// This will create any missing destination sub-directories and ignore the ones that already exist,
/// even if their counterparts also exist in the source directory. Also, this will still not overwrite
/// existing destination files - it will effectively be a merge without overwrites.
///
/// <br>
///
/// If you want files to be overwritten, you may set the behaviour this way:
///
/// ```no_run
/// # use fs_more::directory::DestinationDirectoryRule;
/// # use fs_more::directory::ExistingSubDirectoryBehaviour;
/// # use fs_more::file::ExistingFileBehaviour;
/// let rules = DestinationDirectoryRule::AllowNonEmpty {
///     existing_destination_file_behaviour: ExistingFileBehaviour::Overwrite,
///     existing_destination_subdirectory_behaviour: ExistingSubDirectoryBehaviour::Continue,
/// };
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DestinationDirectoryRule {
    /// Indicates the associated directory function should return an error,
    /// if the destination directory already exists.
    DisallowExisting,

    /// Indicates the associated function should return an error,
    /// if the destination directory exists *and is not empty*.
    AllowEmpty,

    /// Indicates that an existing destination directory should not cause an error,
    /// even if it is not empty.
    ///
    /// **Do not use this as a default if you're not sure what rule to choose.**
    /// This rule can, if the destination directory already has some content,
    /// allow a copy or move that results in a destination directory
    /// with *merged* source and destination directory contents.
    /// Unless you know you want precisely this, you should probably avoid this option.
    ///
    /// Missing destination directories will always be created,
    /// regardless of the `existing_destination_subdirectory_behaviour` option.
    /// Setting it to [`ExistingSubDirectoryBehaviour::Continue`] simply means that,
    /// if they already exist on the destination, nothing special will happen.
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

    pub(crate) fn allows_existing_destination_subdirectories(&self) -> bool {
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



/// Computes a relative path of `source_sub_path` relative to `source_base_directory_path`,
/// and applies it onto `target_base_directory_path`.
///
/// `source_base_directory_path` is the base source directory path,
/// and `source_sub_path` *must* be a descendant of that path.
/// `target_base_directory_path` can be an arbitrary target directory path.
///
/// Returns [`SourceSubPathNotUnderBaseSourceDirectory`]
/// if `source_sub_path` is not a sub-path of `source_base_directory_path`.
///
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
