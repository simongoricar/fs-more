fs-more
=======
[![Crates.io Version](https://img.shields.io/crates/v/fs-more?style=flat-square)](https://crates.io/crates/fs-more)
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.63.0-brightgreen?style=flat-square)](https://github.com/simongoricar/fs-more/blob/master/Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/simongoricar/fs-more/blob/master/LICENSE)
[![Documentation](https://img.shields.io/badge/docs-on%20docs.rs-green?style=flat-square)](https://docs.rs/fs-more)



Convenient file and directory operations built on top of `std::fs` with improved error handling.
Includes copying or moving files and directories with progress reporting.

## Main features
- copying or moving files and directories with in-depth configuration options (including IO buffering settings, copying depth, etc.),
- copying or moving files and directories **with progress reporting**, if needed,
- scanning directories with depth and other options, and
- calculating file or directory sizes.



<br>


## Usage
To add `fs-more` into your project, specify it as a dependency in your `Cargo.toml` file:
```toml
fs-more = "0.3.0"
```


## Examples
Copying a file and getting updates on the progress:

```rust
use std::path::Path;
use fs_more::error::FileError;
use fs_more::file::FileCopyWithProgressOptions;

let source_path = Path::new("./source-file.txt");
let target_path = Path::new("./target-file.txt");

let bytes_copied = fs_more::file::copy_file_with_progress(
    source_path,
    target_path,
    FileCopyWithProgressOptions::default(),
    |progress| {
        let percent_copied =
            (progress.bytes_finished as f64) / (progress.bytes_total as f64)
            * 100.0;
        println!("Copied {:.2}% of the file!", percent_copied);
    }
)?;

println!("Copied {bytes_copied} bytes!");
```

Moving a directory and getting updates on the progress:
```rust
use std::path::Path;
use fs_more::error::DirectoryError;
use fs_more::directory::DirectoryMoveWithProgressOptions;
use fs_more::directory::TargetDirectoryRule;

let source_path = Path::new("./source-directory");
let target_path = Path::new("./target-directory");

let moved = fs_more::directory::move_directory_with_progress(
    source_path,
    target_path,
    DirectoryMoveWithProgressOptions {
        target_directory_rule: TargetDirectoryRule::AllowEmpty,
        ..Default::default()
    },
    |progress| {
        let percent_moved =
            (progress.bytes_finished as f64) / (progress.bytes_total as f64)
            * 100.0;
        println!(
            "Moved {:.2}% of the directory ({} files and {} directories so far).",
            percent_moved,
            progress.files_moved,
            progress.directories_created
        );
    }
)?;

println!(
    "Moved {} bytes ({} files, {} directories)! Underlying strategy: {:?}.",
    moved.total_bytes_moved,
    moved.num_files_moved,
    moved.num_directories_moved,
    moved.used_strategy
);
```

<br>

## Feature flags
The following feature flags are available:
- `fs-err`: enables [`fs-err`](https://docs.rs/fs-err) support, which means more helpful underlying IO error messages
  (though `fs-more` already provides many on its own).
- `miette`: derives [`miette::Diagnostic`](https://docs.rs/miette/latest/miette/derive.Diagnostic.html) on all 
  [error types](https://docs.rs/fs-more/latest/fs_more/error/index.html), 
  allowing users to conveniently call e.g. [`wrap_err`](https://docs.rs/miette/latest/miette/trait.Context.html#tymethod.wrap_err) on the error.


## Project status
This crate evolved out of a general frustration with the `fs_extra` library 
and aims to cover many of the same goals with - hopefully - better documentation and tests.

The majority of features needed in day-to-day use are present.
For now, I plan on keeping the version below `1.0.0` to imply that 
this hasn't gone though a lot.

`fs-more` does lack some thorough battle-testing - as such, use it with a reasonable caution and testing.
However, quite a number of unit, doc and integration tests have been written. 
They cover a wide array of the base functionality, but fringe cases might not be covered yet — 
[contributions](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md) are welcome. The test harness is available in `subcrates/test-harness`.



<br>

## How to contribute
Found a bug or just want to improve `fs-more` by developing new features or writing tests? Awesome!
Start by going over the contribution guide: [`CONTRIBUTING.md`](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md).


### Missing features
> Contributions for the ideas below are most welcome!
>
> Some of these ideas and/or missing features are simpler, some are more of a long shot.
> However, note that even though they are stated below, they probably haven't been thought out deeply enough.
> If you decide to contribute, it would probably be best to first open an issue so various approaches 
> can be discussed before something is developed.

- [ ] *Cross-platform: allow copying file and directory permissions.*

  This partially already exists in some functions, but it inconsistent across the API. 
  The reason is that `std::fs::copy` already copies permission bits, but we don't use that in several places,
  since copying with progress reporting makes using `std::fs::copy` impossible. 
  Ideally, we should expose a new option through the existing `*Options` structs and make this consistent.

  I think this should be reasonably simple to do, but it might take some thinking about edge cases 
  and implementing some platform-specifics (i.e. on Windows, we probably want to copy the hidden file flag, etc).

- [ ] *On Unix: allow copying file and directory owners and groups.*
  
  Depending on how deep the implementation rabbit-hole goes,
  perhaps using [`file-owner`](https://docs.rs/file-owner/latest/file_owner/) or [`nix`](https://docs.rs/nix/latest/nix/)
  could suffice? Perhaps we should feature-gate these kinds of things so the average user doesn't need to pull in so many dependencies?

- [ ] *Cross-platform: allow copying creation/access/modification time of files and directories (across the entire API). 
  This could also include various other metadata.*
  
  Ideally, this should be highly configurable through the existing `*Options` structs.
  This might take some more work though due to various platform differences 
  (see: [Unix](https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html), 
  [Linux](https://doc.rust-lang.org/std/os/linux/fs/trait.MetadataExt.html), 
  [Windows](https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html)).

  It might be more feasible to simply delegate this to some existing crate, 
  i.e. [`filetime`](https://lib.rs/crates/filetime) (but this one covers only timestamps).
  Perhaps we should start with just creation/access/modification timestamps and expand later?

- [ ] *On Windows: allow copying the [ACL](https://learn.microsoft.com/en-us/windows/win32/secauthz/access-control-lists)
  of files and directories.*

  This seems like a long shot and would need some concrete use cases before proceeding. Maybe [`windows-acl`](https://github.com/trailofbits/windows-acl)
  could help? If this feature is to be developed, I think we should not expose any underlying ACL API and allow purely for mirroring it when copying or moving. This should almost definitely be under a feature flag.



<br>

---

### Inspirations

<details>
<summary>Inspired by <code>fs_extra</code></summary>

`fs-more` is very much not a fork, but its API surface has been partially 
inspired by parts of the [`fs_extra`](https://github.com/webdesus/fs_extra) library - thank you!

</details>
