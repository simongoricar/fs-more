use std::{
    fs::{self, FileType},
    path::Path,
};



/// The type of a path, e.g. a file, a symlink to a directory, etc.
///
/// See also: [`PathType::from_path`] or [`PathType::from_path_types`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathType {
    /// The path does not exist.
    NotFound,

    /// The path leads to a "bare" file, i.e. a file *and not a symlink to one*.
    BareFile,

    /// The path leads to a symlink to a file.
    SymlinkToFile,

    BrokenSymlink,

    /// The path leads to a "bare" directory, i.e. a directory *and not a symlink to one*.
    BareDirectory,

    /// The path leads to a symlink to a directory.
    SymlinkToDirectory,

    /// The path exists, but its type is not one of the recognized ones.
    Unrecognized,
}


impl PathType {
    /// Computes the type of a `path`.
    ///
    /// Returns [`std::io::Error`] if the file's metadata cannot be read.
    pub fn from_path<P>(path: P) -> Result<Self, std::io::Error>
    where
        P: AsRef<Path>,
    {
        if !path.as_ref().try_exists()? {
            return Ok(Self::NotFound);
        }

        let metadata_no_follow = fs::symlink_metadata(&path)?;


        if metadata_no_follow.is_symlink() {
            let resolved_symlink_path = fs::read_link(&path)?;

            if !resolved_symlink_path.try_exists()? {
                return Ok(Self::BrokenSymlink);
            }
        }


        let metadata_with_follow = fs::metadata(path)?;

        if metadata_no_follow.is_file() {
            Ok(Self::BareFile)
        } else if metadata_no_follow.is_dir() {
            Ok(Self::BareDirectory)
        } else if metadata_no_follow.is_symlink() {
            if metadata_with_follow.is_file() {
                Ok(Self::SymlinkToFile)
            } else if metadata_with_follow.is_dir() {
                Ok(Self::SymlinkToDirectory)
            } else {
                Ok(Self::Unrecognized)
            }
        } else {
            Ok(Self::Unrecognized)
        }
    }

    /// Computes the type of a path from two [`FileType`]s, one previously obtained from
    /// [`fs::symlink_metadata`], another from [`fs::metadata`] (+ [`Metadata::file_type`]).
    ///
    /// *Unlike [`Self::from_path`], this method does not touch the filesystem.*
    ///
    /// # Invariants
    /// It is up to the caller to ensure the `file_type_no_follow` and `file_type_with_follow`
    /// are actual [`FileType`]s corresponding to [`fs::symlink_metadata`] and [`fs::metadata`] respectively.
    ///
    /// This method exists as a convenience for cases where the metadata has already been obtained,
    /// and needs to be simply turned into [`PathType`].
    ///
    ///
    /// [`Metadata::file_type`]: std::fs::Metadata::file_type
    pub fn from_path_types(file_type_no_follow: FileType, file_type_with_follow: FileType) -> Self {
        if file_type_no_follow.is_file() {
            Self::BareFile
        } else if file_type_no_follow.is_dir() {
            Self::BareDirectory
        } else if file_type_no_follow.is_symlink() {
            if file_type_with_follow.is_file() {
                Self::SymlinkToFile
            } else if file_type_with_follow.is_dir() {
                Self::SymlinkToDirectory
            } else {
                Self::Unrecognized
            }
        } else {
            Self::Unrecognized
        }
    }

    /// Returns a short name of the given path type.
    ///
    /// Examples: "a file", "a symlink to a directory", ...
    ///
    pub fn to_short_name(self) -> &'static str {
        match self {
            PathType::NotFound => "non-existent",
            PathType::BareFile => "a file",
            PathType::SymlinkToFile => "a symlink to a file",
            PathType::BrokenSymlink => "a broken symlink",
            PathType::BareDirectory => "a directory",
            PathType::SymlinkToDirectory => "a symlink to a directory",
            PathType::Unrecognized => "unrecognized",
        }
    }
}
