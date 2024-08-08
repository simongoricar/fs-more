fs-more
=======
[![Crates.io Version](https://img.shields.io/crates/v/fs-more?style=flat-square)](https://crates.io/crates/fs-more)
[![Minimum Supported Rust Version is 1.77.0](https://img.shields.io/badge/MSRV-1.77.0-brightgreen?style=flat-square)](https://releases.rs/docs/1.77.0/)
[![License](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue?style=flat-square)](https://github.com/simongoricar/fs-more/blob/master/LICENSE-MIT)
[![Documentation](https://img.shields.io/badge/docs-published-green?style=flat-square)](https://docs.rs/fs-more)



Convenient file and directory operations built on top of `std::fs` with improved error handling and in-depth configuration.
Includes copying and moving files or directories with progress reporting.


## Main features
- copy and move files or directories with:
  - in-depth configuration options (existing destination file behaviour, copying depth, IO buffer sizes, etc.), and
  - **progress reporting**, if wanted,
- scan directories (with options such as scan depth and symlink behaviour), and
- calculate file or directory sizes.

<br>


## Usage
To add `fs-more` into your project, specify it as a dependency in your `Cargo.toml` file:
```toml
fs-more = "0.7.1"
```


## Examples

Copying a file and getting updates on the progress:

```rust,no_run
use std::path::Path;

use fs_more::file::CollidingFileBehaviour;
use fs_more::file::FileCopyWithProgressOptions;
use fs_more::file::FileCopyFinished;


let source_path = Path::new("./source-file.txt");
let destination_path = Path::new("./destination-file.txt");

let finished_copy = fs_more::file::copy_file_with_progress(
    source_path,
    destination_path,
    FileCopyWithProgressOptions {
        colliding_file_behaviour: CollidingFileBehaviour::Abort,
        ..Default::default()
    },
    |progress| {
        let percent_copied =
            (progress.bytes_finished as f64) 
            / (progress.bytes_total as f64 * 100.0);

        println!("Copied {:.2}% of the file!", percent_copied);
    }
).unwrap();

match finished_copy {
    FileCopyFinished::Created { bytes_copied } => {
        println!("Copied {bytes_copied} bytes into a fresh file!");
    }
    FileCopyFinished::Overwritten { bytes_copied } => {
        println!("Copied {bytes_copied} bytes over an existing file!");
    }
    // ... (see documentation) ...
    _ => {}
};
```

<br>

Moving a directory and getting updates on the progress:

```rust,no_run
use std::path::Path;
use fs_more::directory::DirectoryMoveWithProgressOptions;
use fs_more::directory::DestinationDirectoryRule;


let source_path = Path::new("./source-directory");
let destination_path = Path::new("./destination-directory");

let moved = fs_more::directory::move_directory_with_progress(
    source_path,
    destination_path,
    DirectoryMoveWithProgressOptions {
        destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
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
).unwrap();

println!(
    "Moved {} bytes ({} files, {} directories)! Underlying strategy: {:?}.",
    moved.total_bytes_moved,
    moved.files_moved,
    moved.directories_moved,
    moved.strategy_used
);
```


<br>

## Feature flags
<table>
<thead>
  <tr>
  <th align="left">
<strong><code>dunce</code></strong>
<span style="font-weight: normal">&nbsp;(<i>enabled</i> by default)</span>
  </th>
  </tr>
</thead>
<tbody>
  <tr>
  <td>

Enables the optional support for [`dunce`](https://docs.rs/dunce) which automatically strips Windows' UNC paths
if they can be represented as non-UNC paths (e.g., `\\?\C:\foo` as `C:\foo`). This is done both
internally and in external results from e.g., [`DirectoryScan`](https://docs.rs/fs-more/latest/fs_more/directory/struct.DirectoryScan.html).

This feature is enabled by default — and highly recommended — because path canonicalization on Windows very commonly returns UNC paths.
`dunce` only has an effect when compiling for Windows targets.
  </td>
  </tr>
</tbody>
</table>
  

<table>
<thead>
  <tr>
  <th align="left">
<strong><code>fs-err</code></strong>
<span style="font-weight: normal">&nbsp;(disabled by default)</span>
  </th>
  </tr>
</thead>
<tbody>
  <tr>
  <td>

Enables the optional support for [`fs-err`](https://docs.rs/fs-err) which provides more helpful
error messages for underlying IO errors. It should be noted that `fs-more` does already provide plenty
of context on errors by itself, which is why this is disabled by default.
  </td>
  </tr>
</tbody>
</table>


<br>

## How to contribute
Found a bug or just want to improve `fs-more` by developing new features or writing tests? Awesome!
Start by going over the contribution guide: [`CONTRIBUTING.md`](https://github.com/simongoricar/fs-more/blob/master/CONTRIBUTING.md).


<details>
<summary>🧵 Potential future features</summary>

<br>


Contributions for the ideas below are most welcome!

Some of these ideas and/or missing features are simpler, some are more of a long shot.
However, note that even though they are stated below, they probably haven't been thought out deeply enough.
If you decide to contribute, it would probably be best to first open an issue, 
so various approaches can be discussed before something is developed.

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
  could help? If this feature is to be developed, I think we should not expose any underlying ACL API and allow purely for mirroring it when copying or moving. This should definitely be under a feature flag.

</details>


<br>

---

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
