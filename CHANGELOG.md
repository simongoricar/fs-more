# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).



## [Unreleased]

This is a pretty significant release with a large set of breaking changes (yet again).

Alongside several fields, parameters, and types being renamed, there are three important additions and changes to point out:
- `DirectoryScan` has been removed in favor of the new iterator-based `DirectoryScanner` inspired by the wonderful `walkdir` crate,
- directory copy and move function now have options that specify how to behave when encountering valid as well as broken symbolic links,
- the `file_size_in_bytes` function no longer follows symbolic links.


### Added
- Added new iterator-based directory scanner (see `DirectoryScanner`; the old scanner has been removed).
  The same data can be acquired as before, the only difference is that the new iterator 
  will yield individual entries, so you'll need to sort e.g. files and directories yourself.
- Directory copy and move functions now have two new options: `symlink_behaviour` and `broken_symlink_behaviour`.
  Callers can now customize whether symbolic links are preserved in the destination, or followed (dereferenced),
  as well as customize how `fs-more` should behave when broken symbolic links are encountered.
- The internal test harness now has support for generating trees with broken symbolic links, 
  so we can properly test that sort of behaviour.
- Directory move functions have two possible strategies: rename and copy-and-delete (this is not new). 
  The new thing are the newly-added options that control which strategies the function may use.
  For example, you may now configure `move_directory` to perform a move only by renaming, only by copy-and-deleting,
  or by trying to rename with copy-and-deleting as a fallback (this is the default, which is the same as in previous versions).
  This set of options is provided as a convenience for unusual cases - in the vast majority of situations, stick to `Either`,
  which will automatically select the best strategy.


### Changed
- `ExistingSubDirectoryBehaviour` has been renamed to `CollidingSubDirectoryBehaviour`, and its appearance in
  fields and parameters has been renamed from `existing_destination_subdirectory_behaviour` to `colliding_subdirectory_behaviour`.
- `ExistingFileBehaviour` has been renamed to `CollidingFileBehaviour`, and its appearance in
  fields and parameters has been renamed from `existing_destination_file_behaviour` to `colliding_file_behaviour`.
- `CopyDirectoryDepthLimit` has been renamed to `DirectoryCopyDepthLimit` to be in line with the type naming scheme elsewhere.
- The `file_size_in_bytes` function no longer follows symbolic links, and instead returns the size of the link itself.
- Option struct fields for directory move functions have been shuffled around - certain options have been moved.


### Fixed
- Reported in #1: `move_directory` and `move_directory_with_progress` now correctly attempt to move a directory by renaming it
  when the destination doesn't exist; the underlying code previously never triggered due to an improper condition.
- Reported in #2: directory copy and move functions no longer refuse to operate on directories containing broken symbolic links,
  and instead respect the provided `broken_symlink_behaviour` option. 
  By default, any broken symbolic links will be kept as is, i.e. broken.
- `move_directory` and `move_directory_with_progress` no longer erroneously rename the symlink destination
  instead of the symlink itself when the provided source path is a symlink to a directory.
- Internal `Path::try_exists` calls have been preventatively migrated over to a custom internal 
  `try_exists_without_follow` that does not follow symbolic links, in order to avoid edge cases 
  when encountering broken symbolic links (none known, but better safe than sorry).


### Removed
- Removed old directory scanner in favor of the new iterator-based one.


---

## [0.6.1] - 2024-07-10

### Added
- Added tests for `is_directory_empty` to avoid such large errors.

### Changed
- Updated directory module documentation to include `DirectoryScan`.

### Fixed
- Fixed `is_directory_empty` returning incorrect results.
- Fixed invalid link to `DirectoryScan` in `README.md`.



## [0.6.0] - 2024-06-16

### Changed
- Updated `file` and `directory` module documentation with tables showing an overview of the features.
- Reworded some documentation and improved how feature flags are structured in the documentation.

### Removed
- The `use_enabled_fs_module!` macro is no longer visible externally (it was not meant to be used anyway).

### Fixed
- Fixed source file paths not being validated properly. They are now always canonicalized before proceeding.
- Fixed tests incorrectly comparing paths. We now attempt to strip UNC prefixes from paths when comparing them. 
  This way the tests do not depend on the `dunce` feature flag being enabled.



## [0.5.1] - 2024-06-15

### Added
- Doctests in `README.md` are now included when testing, including in CI, to keep those examples from going out of date.

### Fixed
- Fixed code examples in `README.md` that previously wouldn't compile.
- Fixed GitHub Pages publishing workflow (previously redirected to a sub-page incorrectly).



## [0.5.0] - 2024-06-14

This is a rather big (and breaking) release - a substantial amount of the API surface has been reworked.

Several structs have been renamed or changed, and several were freshly introduced, 
which will mean having to manually go through a bit of code to migrate if you're on `v0.4.0`.
This changelog likely doesn't cover all of the changes that the crate got, but hopefully most of them.


### Added
- The `dunce` feature flag is now available and enabled by default.
  It brings in the [`dunce`](https://docs.rs/dunce) crate, 
  which automatically strips Windows' UNC paths if they can be represented
  using the usual type of path (e.g. `\\?\C:\foo -> C:\foo`).
  This is used both internally and in e.g. `DirectoryScan`'s file and directory paths. 
  This feature flag is enabled by default and recommended because path canonicalization 
  very commonly returns UNC paths. This crate only has an effect when compiling for Windows targets.
- `fs-more` is now available under the `Apache-2.0` license as well! 
  Our new license expression is therefore `MIT OR Apache-2.0`.
- Several options throughout the crate have been refactored, for example:
  - When copying a directory, you can specify whether you allow the destination directory to exist, 
    and if so, whether it can be non-empty (this is not new). What is new is that you can more precisely specify 
    what actions to take when encountering file or subdirectory collisions (see `DestinationDirectoryRule`).
- Behaviour with symbolic links for all functions has been reviewed (and added to integration tests), 
  and the relevant documentation has been updated.
- `DirectoryScan` now has a standalone options struct, and its symlink behaviour has received more testing.
- Several file-related functions, such as `copy_file`, now return a standalone struct (e.g. `FileCopyFinished`) 
  containing additional context about the actions performed, such as whether the file was created, overwritten, or skipped.
- Error types now have better documentation, including explanations for common errors
  in the documentation on main functions themselves (see e.g. `copy_directory_with_progress`).


### Changed
- `fs-more`'s MSRV has been increased to `1.77.0` due to our use of `File::create_new`.
- Most mentions of a "target" file or directory are now referred to as a "destination" file or directory.
- The buffer sizes for reading and writing have been split from one into two options.
- The progress update interval's default value of 64 KiB has been increased to a larger, and not really any less useful, 512 KiB,
  to avoid potential performance problems. Calling a closure every 64 KiB can be either excessive or a choice, but
  it's definitely not a good default.
- The error types have been fully overhauled and now allow much better insight into the actual error type 
  as well as context surrounding it. Some would probably say that the error types are actually *too complex*, 
  but a middle ground seems hard to achieve. So for now, when in doubt, just convert them into generic errors 
  with e.g. `dyn Error` trait objects or `miette`'s `into_diagnostic`.
- The testing harness has received a full overhaul. Not only are there more assertions available for easier testing,
  but the filesystem tree harness has been fully rewritten: we now declare the file tree in a JSON file,
  and the corresponding Rust code is generated from that schema. Each tree harness is responsible for temporarily initializing 
  the file tree, and allows us to inspect individual components.
  More details are available in [`CONTRIBUTING.md`](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md).
- To create a consistent naming scheme, several structs have been renamed. Previously, things
  like `FinishedDirectoryMove` existed, but now we adhere to `[type][operation][name]`, e.g. `FileMoveOptions`. Examples:
  - `TargetDirectoryRule` has been renamed to `DestinationDirectoryRule`.
  - `FinishedDirectoryCopy` has been renamed to `DirectoryCopyFinished`.
  - `FinishedDirectoryMove` has been renamed to `DirectoryMoveFinished`.
  - `DirectoryCopyProgress` has been renamed to `DirectoryCopyProgressRef` 
    (`DirectoryCopyProgress` exists in this version, but that's an owned version of `DirectoryCopyProgressRef`).
- Several field names have been reworded, e.g. `num_files_copied` is now `files_copied`.
- `DirectoryScan` no longer exposes the `files` and `directories` fields directly - see the `files()` and `directories()` methods instead.
- Depth limits are now generally enums, not `Option<usize>` (see `CopyDirectoryDepthLimit`, among others).
- Many internals have been reorganised for easier maintainability and less code repetition.
- Integration tests have been reorganised from separate binaries into a single `integration` binary for compilation speed.
  Additionally, the tests have been restructured into smaller modules for clarity.


### Removed
- The `miette` feature flag has been removed. It previously just derived `Diagnostic` on all error types,
  we did not take advantage of anything that the end user could not do with `into_diagnostic` themselves.
- The `fs_more_test_harness_macros` procedural macro crate has been removed in favor of the new
  `fs_more_test_harness_tree_generator` CLI crate, which is part of the new test harness.



## [0.4.0] - 2024-03-13

### Changed
- Upgrade outdated dependencies.
- Improve documentation and contribution guide.
- Refactor internal test harness subcrates.
  
### Removed
- `TargetDirectoryRule` no longer publicly exposes the three methods that previously returned a bool indicating its existing target directory and overwriting behaviour.


## [0.3.0] - 2024-01-14

### Added
- Add `miette` feature flag that derives `miette::Diagnostic` on all error types,
  enabling covenient `wrap_err_with` and the like.



## [0.2.2] - 2024-01-13

### Fixed
- Fix `README.md` to correctly state library usage via `crates.io`, not Git.



## [0.2.1] - 2024-01-13

### Fixed
- Fix `license` field in `Cargo.toml`



## [0.2.0] - 2024-01-13

Initial development release with the API mostly stable.



[Unreleased]: https://github.com/simongoricar/fs-more/compare/v0.6.0...HEAD
[0.6.1]: https://github.com/simongoricar/fs-more/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/simongoricar/fs-more/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/simongoricar/fs-more/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/simongoricar/fs-more/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/simongoricar/fs-more/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/simongoricar/fs-more/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/simongoricar/fs-more/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/simongoricar/fs-more/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/simongoricar/fs-more/compare/727e90a7ff9c70359fb9a4a5ebdf5e5e528f4708...v0.2.0
