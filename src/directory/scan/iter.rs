use std::{
    collections::VecDeque,
    fs::Metadata,
    path::{Path, PathBuf},
};

use_enabled_fs_module!();

use super::{DirectoryScanDepthLimit, DirectoryScanOptionsV2, ScanEntry};
use crate::{directory::ScanEntryDepth, error::DirectoryScanErrorV2};


/// A currently open directory that is being iterated over (scanned).
struct OpenDirectory {
    /// Path of the directory being scanned.
    directory_path: PathBuf,

    /// Depth of the directory, relative to the root of the scan tree.
    directory_depth: ScanEntryDepth,

    /// The actual directory iterator from the standard library.
    iterator: fs::ReadDir,
}


/// A directory pending for a scan.
struct PendingDirectory {
    /// Path of the directory to be scanned.
    directory_path: PathBuf,

    /// Depth of the directory, relative to the root of the scan tree.
    directory_depth: ScanEntryDepth,
}


/// Represents a file tree ancestor.
///
/// Used for symlink cycle detection when
/// [`DirectoryScanOptionsV2::should_track_ancestors`] returns `true`.
struct Ancestor {
    /// Path of the ancestor directory.
    path: PathBuf,

    /// Depth of the ancestor directory.
    depth: ScanEntryDepth,
}


/// Internal information about the next yielded directory entry.
struct NextEntryInfo {
    /// Path of the next entry.
    ///
    /// If [`DirectoryScanOptionsV2::follow_symbolic_links`] is `true`,
    /// this path will never lead to a symlink.
    path: PathBuf,

    /// Metadata regarding the entry.
    metadata: Metadata,

    /// Depth of the entry, relative to the root of the scan tree.
    depth: ScanEntryDepth,
}



/// A recursive breadth-first directory iterator.
///
/// Obtained from calling `into_iter` after
/// initializing the directory scanner ([`DirectoryScanner::new`]).
///
///
/// [`DirectoryScanner::new`]: super::DirectoryScanner::new
pub struct BreadthFirstDirectoryIter {
    /// Path of the directory the scan started at.
    base_directory: PathBuf,

    /// Directory scanning options.
    options: DirectoryScanOptionsV2,

    /// Whether the `base_directory` has been added to the pending scan queue yet.
    /// If the base directory is a symbolic link to a directory and
    /// [`follow_base_directory_symbolic_link`] is `true`,
    /// the symlink is also resolved in this step, and the `base_directory` field value
    /// is updated to reflect the symlink destination.
    ///
    /// This is generally done on the first call to [`Self::next`].
    ///
    /// [`follow_base_directory_symbolic_link`]: DirectoryScanOptionsV2::follow_base_directory_symbolic_link
    has_processed_base_directory: bool,

    /// Whether the `base_directory` has been taken from the pending scan queue and opened for reading yet.
    ///
    /// This is generally done on the first call to [`Self::next`].
    has_scanned_base_directory: bool,

    /// A stack of pending directory paths that have yet to be scanned.
    ///
    /// Fresh entries are added to the back, and the next entry is taken from the front (FIFO order).
    pending_directory_stack: VecDeque<PendingDirectory>,

    /// If `Some`, this field contains the currently active directory "reader" (iterator),
    /// along with information about the directory path we're scanning, and its depth in the scan tree.
    currently_open_directory: Option<OpenDirectory>,

    /// Contains a list of directory paths used for preventing symbolic link cycles.
    /// The values are ancestors of the `currently_open_directory`, plus the current directory itself
    /// (which is the last element). Items are therefore ordered from shallowest to deepest
    /// (i.e. first element is a handle to the base directory).
    ///
    /// This will always be empty if [`follow_symbolic_links`] is `false`
    /// (i.e. when [`DirectoryScanOptionsV2::should_track_ancestors`] returns `false`).
    ///
    ///
    /// [`follow_symbolic_links`]: DirectoryScanOptionsV2::follow_symbolic_links
    current_directory_ancestors: Vec<Ancestor>,
}


impl BreadthFirstDirectoryIter {
    pub(super) fn new<P>(base_directory: P, options: DirectoryScanOptionsV2) -> Self
    where
        P: Into<PathBuf>,
    {
        let base_directory: PathBuf = base_directory.into();

        Self {
            base_directory,
            has_processed_base_directory: false,
            has_scanned_base_directory: false,
            options,
            currently_open_directory: None,
            pending_directory_stack: VecDeque::new(),
            current_directory_ancestors: vec![],
        }
    }


    /// Returns a reference to the currently active (open) directory iterator, if any,
    /// `None` otherwise.
    fn current_directory_handle(&self) -> Option<&OpenDirectory> {
        if self.currently_open_directory.is_some() {
            let handle = self
                .currently_open_directory
                .as_ref()
                .expect("currently_open_directory should be Some");

            return Some(handle);
        }

        None
    }

    /// Returns the directory path of the currently active (open) directory iterator.
    ///
    /// # Panics
    /// If there is no currently open directory iterator, this method will panic.
    /// It is up to the caller to ensure a directory iterator iss be currently active.
    fn current_directory_handle_path_unchecked(&self) -> &Path {
        &self
            .current_directory_handle()
            .expect("expected a directory handle to be open")
            .directory_path
    }

    /// Returns the entry depth of the currently active (open) directory iterator.
    ///
    /// # Panics
    /// If there is no currently open directory iterator, this method will panic.
    /// It is up to the caller to ensure a directory iterator iss be currently active.
    fn current_directory_handle_depth_unchecked(&self) -> &ScanEntryDepth {
        &self
            .current_directory_handle()
            .expect("expected a directory handle to be open")
            .directory_depth
    }

    /// Returns a mutable reference to a directory iterator, either the current one,
    /// or if none is active at the moment, opening the next directory iterator on the stack.
    ///
    /// Returns `None` if the pending directory stack is empty, i.e. when the iterator has
    /// no more elements to scan and yield.
    fn current_or_next_directory_handle_mut(
        &mut self,
    ) -> Result<Option<&mut OpenDirectory>, DirectoryScanErrorV2> {
        if self.currently_open_directory.is_some() {
            let handle = self
                .currently_open_directory
                .as_mut()
                .expect("currently_open_directory should be Some");

            return Ok(Some(handle));
        }

        self.open_next_directory_handle()
    }

    /// Returns a [`SymlinkCycleEncountered`] error if the provided `directory_path`
    /// would lead to a scan cycle, e.g. when symlinks are cyclic.
    ///
    /// This method relies on the [`Self::current_directory_ancestors`] field
    /// to be properly maintained as directories are entered or exited.
    ///
    ///
    /// # Invariants
    /// - `directory_path` must not be a symlink to a directory (you must resolve the link yourself
    ///   before calling the function).
    ///
    ///
    /// [`SymlinkCycleEncountered`]: DirectoryScanErrorV2::SymlinkCycleEncountered
    fn ensure_directory_path_does_not_lead_to_a_tree_cycle(
        &self,
        directory_path: &Path,
    ) -> Result<(), DirectoryScanErrorV2> {
        for ancestor_directory_handle in &self.current_directory_ancestors {
            if directory_path.eq(&ancestor_directory_handle.path) {
                return Err(DirectoryScanErrorV2::SymlinkCycleEncountered {
                    directory_path: directory_path.to_path_buf(),
                });
            }
        }

        Ok(())
    }

    /// Attempts to open the next pending directory for iteration.
    ///
    /// On success, a `Ok(Some(&mut`[`OpenDirectory`]`))` is returned,
    /// containing the newly-opened directory which is ready for iteration.
    ///
    /// **Note that if a directory is already open when calling this,
    /// it will be discarded!** Prefer using [`Self::close_current_directory_handle`]
    /// when a directory is exhausted instead of directly calling this method.
    ///
    /// # Edge cases
    /// If there is no pending directory left, and the base directory has already been scanned,
    /// `Ok(None)` is returned.
    fn open_next_directory_handle(
        &mut self,
    ) -> Result<Option<&mut OpenDirectory>, DirectoryScanErrorV2> {
        if !self.has_scanned_base_directory {
            // We've just started, perhaps having just yielded the base directory.
            // As such, we should open the base directory.

            let base_dir_iterator = fs::read_dir(&self.base_directory).map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectory {
                    directory_path: self.base_directory.clone(),
                    error: io_error,
                }
            })?;


            let active_reader_entry = OpenDirectory {
                directory_path: self.base_directory.clone(),
                directory_depth: ScanEntryDepth::BaseDirectory,
                iterator: base_dir_iterator,
            };


            assert!(self.currently_open_directory.is_none());

            self.currently_open_directory = Some(active_reader_entry);
            self.has_scanned_base_directory = true;


            if self.options.should_track_ancestors() {
                #[cfg(debug_assertions)]
                {
                    if !self.current_directory_ancestors.is_empty() {
                        panic!(
                            "expected current_directory_ancestors to be empty \
                            before opening and scanning the base directory"
                        );
                    }
                }

                self.current_directory_ancestors.push(Ancestor {
                    path: self.base_directory.clone(),
                    depth: ScanEntryDepth::BaseDirectory,
                });
            }


            let handle_mut = self.currently_open_directory
                .as_mut()
                // PANIC SAFETY: We just `push`-ed onto the vector, meaning this will never panic.
                .expect("currently_open_directory should be Some");

            return Ok(Some(handle_mut));
        }


        // The base directory has already been opened or read;
        // open one pending directory from the pending directory stack instead,
        // or return `None` if no pending directories are left.
        let Some(next_pending_directory) = self.pending_directory_stack.pop_front() else {
            return Ok(None);
        };


        let directory_iterator =
            fs::read_dir(&next_pending_directory.directory_path).map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectory {
                    directory_path: next_pending_directory.directory_path.clone(),
                    error: io_error,
                }
            })?;

        let active_reader_entry = OpenDirectory {
            directory_path: next_pending_directory.directory_path.clone(),
            directory_depth: next_pending_directory.directory_depth,
            iterator: directory_iterator,
        };



        // This will also drop the previously open directory.
        // It is up to the caller of this function to ensure this is wanted behaviour.
        self.currently_open_directory = Some(active_reader_entry);


        // We always need to pop at most one ancestor, and only when
        // moving to scan an adjacent directory with the same depth.
        // We never need to pop more, because we're doing a breadth-first scan.
        // As such, the depth is guaranteed to be monotonically increasing.
        //
        // For example:
        // ```
        // a
        // |- b
        // |  |-> c.bin
        // |  |-> d.bin
        // |- e
        //    |- f
        //    |  |-> g.bin
        //    |- h
        // ```
        //
        // As far as cycle detection is concerned in such a tree,
        // we'll need to track the ancestors in the following way:
        // - before entering (scanning) `a`, append `a` to the ancestor list,
        // - before entering `b`, append `b` to the ancestor list,
        // - before entering `e`, pop `b` and append `e` to the ancestor list,
        // - before entering `f`, append `f` to the ancestor list,
        // - before entering `h`, pop `f` and append `h` to the ancestor list.
        //
        // As you can see, we only need to pop a single ancestor when moving through
        // adjacent directories of the same depth.
        if self.options.should_track_ancestors() {
            if let Some(last_ancestor) = self.current_directory_ancestors.last() {
                if last_ancestor
                    .depth
                    .eq(&next_pending_directory.directory_depth)
                {
                    self.current_directory_ancestors.pop();
                }
            }

            self.current_directory_ancestors.push(Ancestor {
                path: next_pending_directory.directory_path,
                depth: next_pending_directory.directory_depth,
            });
        }



        let handle_mut = self.currently_open_directory
            .as_mut()
            // PANIC SAFETY: We just `push`-ed onto the vector.
            .expect("currently_open_directory should be Some");

        Ok(Some(handle_mut))
    }

    /// This method will close the current directory handle,
    /// or return an `Err(())` if there is no open handle.
    fn close_current_directory_handle(&mut self) -> Result<(), ()> {
        let Some(_) = self.currently_open_directory.take() else {
            return Err(());
        };

        Ok(())
    }

    /// Pushes a directory onto the pending directory scan queue.
    /// As this is a breadth-first iterator, the new directory will be placed last.
    fn queue_directory_for_scanning(&mut self, directory_path: PathBuf, depth: ScanEntryDepth) {
        let pending_dir_entry = PendingDirectory {
            directory_path,
            directory_depth: depth,
        };

        self.pending_directory_stack.push_back(pending_dir_entry);
    }

    /// Returns the next directory scan entry. This will automatically manage
    /// the currently open directory, as well as close and open new pending directories, etc.
    ///
    /// If the scan has been exhausted, `Ok(None)` is returned, signalling the end of the iterator.
    ///
    /// If following symlinks is enabled, the returned entries will have their symlink paths followed.
    fn next_entry(&mut self) -> Result<Option<NextEntryInfo>, DirectoryScanErrorV2> {
        loop {
            let follow_symbolic_links = self.options.follow_symbolic_links;

            let Some(current_directory_iterator) = self.current_or_next_directory_handle_mut()?
            else {
                return Ok(None);
            };


            let Some(raw_entry_result) = current_directory_iterator.iterator.next() else {
                self.close_current_directory_handle()
                        // PANIC SAFETY: We just held a reference to an open directory,
                        // which means `close_current_directory_handle` will be able to remove it, 
                        // as it does exist.
                        .expect("at least one directory should be currently opened");

                // The loop will restart and a new directory will be opened.
                // If there are no further directories to scan, `None` will be returned from the iterator.

                continue;
            };



            let raw_entry = raw_entry_result.map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                    directory_path: self
                                .current_directory_handle()
                                // PANIC SAFETY: between the call to `current_or_next_directory_handle_mut` and this point,
                                // we never call `close_current_directory_handle`.
                                .expect("expected a directory handle to be open")
                                .directory_path
                                .clone(),
                    error: io_error,
                }
            })?;

            let raw_entry_metadata = raw_entry.metadata().map_err(|io_error| {
                DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                    // PANIC SAFETY: between the call to `current_or_next_directory_handle_mut` and this point,
                    // we never call `close_current_directory_handle`.
                    directory_path: self.current_directory_handle_path_unchecked().to_path_buf(),
                    error: io_error,
                }
            })?;


            let (raw_entry_path, raw_entry_metadata) =
                if follow_symbolic_links && raw_entry_metadata.is_symlink() {
                    let resolved_raw_entry_path =
                        fs::read_link(raw_entry.path()).map_err(|io_error| {
                            DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                // PANIC SAFETY: between the call to `current_or_next_directory_handle_mut` and this point,
                                // we never call `close_current_directory_handle`.
                                directory_path: self
                                    .current_directory_handle_path_unchecked()
                                    .to_path_buf(),
                                error: io_error,
                            }
                        })?;

                    let raw_entry_metadata_followed =
                        fs::symlink_metadata(&resolved_raw_entry_path).map_err(|io_error| {
                            DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                // PANIC SAFETY: between the call to `current_or_next_directory_handle_mut` and this point,
                                // we never call `close_current_directory_handle`.
                                directory_path: self
                                    .current_directory_handle_path_unchecked()
                                    .to_path_buf(),
                                error: io_error,
                            }
                        })?;


                    if raw_entry_metadata_followed.is_dir() {
                        // Function invariant upheld: `resolved_raw_entry_path` is the symlink destination.
                        self.ensure_directory_path_does_not_lead_to_a_tree_cycle(
                            &resolved_raw_entry_path,
                        )?;
                    }

                    (resolved_raw_entry_path, raw_entry_metadata_followed)
                } else {
                    (raw_entry.path(), raw_entry_metadata)
                };


            return Ok(Some(NextEntryInfo {
                path: raw_entry_path,
                metadata: raw_entry_metadata,
                // PANIC SAFETY: between the call to `current_or_next_directory_handle_mut` and this point,
                // we never call `close_current_directory_handle`.
                depth: self
                    .current_directory_handle_depth_unchecked()
                    .plus_one_level(),
            }));
        }
    }
}



impl Iterator for BreadthFirstDirectoryIter {
    type Item = Result<ScanEntry, DirectoryScanErrorV2>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_processed_base_directory {
            self.has_processed_base_directory = true;

            // Follow symlink if configured to do so.
            let base_directory_metadata =
                try_some!(fs::symlink_metadata(&self.base_directory), |io_error| {
                    DirectoryScanErrorV2::UnableToReadDirectory {
                        directory_path: self.base_directory.clone(),
                        error: io_error,
                    }
                });


            if !base_directory_metadata.is_symlink() && !base_directory_metadata.is_dir() {
                return Some(Err(DirectoryScanErrorV2::NotADirectory {
                    path: self.base_directory.clone(),
                }));
            }

            if base_directory_metadata.is_symlink() {
                if !self.options.yield_base_directory {
                    // Nothing no follow, nothing to yield - the iterator will have no elements.
                    return None;
                }

                let symlink_destination =
                    try_some!(fs::read_link(&self.base_directory), |io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectory {
                            directory_path: self.base_directory.clone(),
                            error: io_error,
                        }
                    });

                let symlink_destination_metadata =
                    try_some!(fs::symlink_metadata(&symlink_destination), |io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectory {
                            directory_path: self.base_directory.clone(),
                            error: io_error,
                        }
                    });

                if !symlink_destination_metadata.is_dir() {
                    return Some(Err(DirectoryScanErrorV2::NotADirectory {
                        path: self.base_directory.clone(),
                    }));
                }


                // We followed the symlink, and we should now update our iterator's base directory path.
                self.base_directory = symlink_destination;
            }


            if self.options.yield_base_directory {
                return Some(Ok(ScanEntry::new(
                    self.base_directory.clone(),
                    base_directory_metadata,
                    ScanEntryDepth::BaseDirectory,
                )));
            }
        }


        let next_entry = {
            let Some(next_entry_info) = try_some!(self.next_entry()) else {
                // No further entries, the iterator has concluded. Once this is reached,
                // all subsequent calls to `next` will also hit this branch, returning `None`.
                return None;
            };


            if next_entry_info.metadata.is_dir() {
                let ScanEntryDepth::AtDepth {
                    depth: current_dir_depth,
                } = next_entry_info.depth
                else {
                    // PANIC SAFETY: Only the base directory can be emitted with `ScanEntryDepth::BaseDirectory`,
                    // and the code flow ensures it's not in this branch.
                    panic!("expected the next entry's depth to be 0+, not base directory");
                };

                match self.options.maximum_scan_depth {
                    DirectoryScanDepthLimit::Unlimited => {
                        self.queue_directory_for_scanning(
                            next_entry_info.path.clone(),
                            next_entry_info.depth,
                        );
                    }
                    DirectoryScanDepthLimit::Limited { maximum_depth } => {
                        if current_dir_depth < maximum_depth {
                            self.queue_directory_for_scanning(
                                next_entry_info.path.clone(),
                                next_entry_info.depth,
                            );
                        }
                    }
                }
            }


            ScanEntry::new(next_entry_info.path, next_entry_info.metadata, next_entry_info.depth)
        };


        Some(Ok(next_entry))
    }
}
