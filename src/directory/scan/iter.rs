use std::{collections::VecDeque, fs::Metadata, path::PathBuf};

use_enabled_fs_module!();

use super::{DirectoryScanDepthLimit, DirectoryScanOptionsV2, ScanEntry};
use crate::{directory::ScanEntryDepth, error::DirectoryScanErrorV2};


struct OpenDirectory {
    directory_path: PathBuf,

    directory_depth: ScanEntryDepth,

    iterator: fs::ReadDir,
}

struct PendingDirectory {
    directory_path: PathBuf,

    directory_depth: ScanEntryDepth,
}


struct NextEntryInfo {
    path: PathBuf,
    metadata: Metadata,
    followed_symlink: bool,
    depth: ScanEntryDepth,
}



pub struct DirectoryScannerPerDirectoryIter {
    base_directory: PathBuf,

    has_processed_base_directory: bool,

    has_scanned_base_directory: bool,

    options: DirectoryScanOptionsV2,

    currently_open_directory: Option<OpenDirectory>,

    pending_directory_stack: VecDeque<PendingDirectory>,
}

impl DirectoryScannerPerDirectoryIter {
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
        }
    }

    fn current_directory_handle_mut(
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


            let handle_mut = self.currently_open_directory
                .as_mut()
                // PANIC SAFETY: We just `push`-ed onto the vector.
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
            directory_path: next_pending_directory.directory_path,
            directory_depth: next_pending_directory.directory_depth,
            iterator: directory_iterator,
        };

        // This will also drop the previously open directory.
        // It is up to the caller of this function to ensure this is wanted behaviour.
        self.currently_open_directory = Some(active_reader_entry);


        let handle_mut = self.currently_open_directory
            .as_mut()
            // PANIC SAFETY: We just `push`-ed onto the vector.
            .expect("currently_open_directory should be Some");

        Ok(Some(handle_mut))
    }

    fn close_current_directory_handle(&mut self) -> Result<(), ()> {
        self.currently_open_directory.take().map(|_| ()).ok_or(())
    }

    fn queue_directory_for_scanning(&mut self, directory_path: PathBuf, depth: ScanEntryDepth) {
        let pending_dir_entry = PendingDirectory {
            directory_path,
            directory_depth: depth,
        };
        self.pending_directory_stack.push_back(pending_dir_entry);
    }

    fn next_entry(&mut self) -> Result<Option<NextEntryInfo>, DirectoryScanErrorV2> {
        loop {
            let follow_symbolic_links = self.options.follow_symbolic_links;

            let Some(current_directory_iterator) = self.current_directory_handle_mut()? else {
                return Ok(None);
            };


            match current_directory_iterator.iterator.next() {
                Some(raw_entry_result) => {
                    let raw_entry = raw_entry_result.map_err(|io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                            directory_path: current_directory_iterator.directory_path.clone(),
                            error: io_error,
                        }
                    })?;

                    let raw_entry_metadata = raw_entry.metadata().map_err(|io_error| {
                        DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                            directory_path: current_directory_iterator.directory_path.clone(),
                            error: io_error,
                        }
                    })?;


                    let (raw_entry_path, raw_entry_metadata, raw_entry_followed_symlink) =
                        if follow_symbolic_links && raw_entry_metadata.is_symlink() {
                            let resolved_raw_entry_path =
                                fs::read_link(raw_entry.path()).map_err(|io_error| {
                                    DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                        directory_path: current_directory_iterator
                                            .directory_path
                                            .clone(),
                                        error: io_error,
                                    }
                                })?;

                            let raw_entry_metadata_followed =
                                fs::symlink_metadata(&resolved_raw_entry_path).map_err(
                                    |io_error| DirectoryScanErrorV2::UnableToReadDirectoryEntry {
                                        directory_path: current_directory_iterator
                                            .directory_path
                                            .clone(),
                                        error: io_error,
                                    },
                                )?;

                            (resolved_raw_entry_path, raw_entry_metadata_followed, true)
                        } else {
                            (raw_entry.path(), raw_entry_metadata, false)
                        };


                    return Ok(Some(NextEntryInfo {
                        path: raw_entry_path,
                        metadata: raw_entry_metadata,
                        followed_symlink: raw_entry_followed_symlink,
                        depth: current_directory_iterator.directory_depth.plus_one_level(),
                    }));
                }
                None => {
                    self.close_current_directory_handle()
                        // PANIC SAFETY: We just held a reference to an open directory,
                        // which means `close_current_directory_handle` will be able to remove it, 
                        // as it does exist.
                        .expect("at least one directory should be currently opened");

                    // The loop will restart and a new directory will be opened.
                    // If there are no further directories to scan, `None` will be returned from the iterator.
                }
            }
        }
    }
}



impl Iterator for DirectoryScannerPerDirectoryIter {
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
