fs-more
=======
[![Crates.io Version](https://img.shields.io/crates/v/fs-more)](https://crates.io/crates/fs-more)
![MSRV](https://img.shields.io/badge/MSRV-1.63.0-brightgreen)
[![License](https://img.shields.io/badge/license-MIT-blue)](https://github.com/simongoricar/fs-more/blob/master/LICENSE)
[![Documentation link](https://img.shields.io/badge/docs-on%20docs.rs-green?style=flat)](https://docs.rs/fs-more)



Convenient file and directory operations built on top of `std::fs` with improved error handling.
Includes copying / moving files and directories with progress reporting.

## Main features
- copying and moving files or directories with in-depth configuration options (including IO buffering settings, copying depth, etc.),
- optionally, copying and moving files or directories **with progress reporting**,
- scanning directories with depth and other options, and
- calculating file or directory sizes.



<br>


## Usage
To add `fs-more` into your project, specify it in your `Cargo.toml` file:
```toml
fs-more = "0.2.2"
```


<br>

## Examples
Copying a file with a progress handler:

```rust
use std::path::Path;
use fs_more::error::FileError;
use fs_more::file::FileCopyWithProgressOptions;

let source_path = Path::new("./source-file.txt");
let target_path = Path::new("./target-file.txt");
let copy_result = fs_more::file::copy_file_with_progress(
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
```

Moving a directory with a progress handler:
```rust
use std::path::Path;
use fs_more::error::DirectoryError;
use fs_more::directory::DirectoryMoveWithProgressOptions;
use fs_more::directory::TargetDirectoryRule;

let source_path = Path::new("./source-directory");
let target_path = Path::new("./target-directory");
let move_result = fs_more::directory::move_directory_with_progress(
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
```

<br>

## Feature flags
The following feature flags are available:
- `fs-err`: enables the optional [`fs-err`](https://docs.rs/fs-err) support, enabling more helpful underlying IO error messages
  (though `fs-more` already provides many on its own).


## Project status
This crate lacks some more thorough battle-testing. 
As such, use it with reasonable caution and testing.

Most features have been added, but it is possible some smaller ones will turn up missing.
For now, I plan on keeping the version below `1.0.0` to imply that 
this hasn't gone though a lot.

However, quite a few unit, doc- and integration tests have been written. 
They cover a wide array of the base functionality, but fringe cases might not be covered yet — 
[contributions](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md) are welcome! 

## Contributing and development
Want to contribute? Awesome!
Start by going over the contribution guide: [`CONTRIBUTING.md`](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md).




---

### Attribution
<details>
<summary>Inspired by <code>fs_extra</code></summary>

`fs-more` isn't a fork, but has been inspired by
some of the functionalities of the [`fs_extra`](https://github.com/webdesus/fs_extra) library (thank you!).

</details>