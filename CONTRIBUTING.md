`fs-more` Contribution Guide
============================

<br>

#### Table of Contents
- [1. Ways to Contribute](#1-ways-to-contribute)
  - [1.1 Bug reporting](#11-bug-reporting)
  - [1.2 Developing features](#12-developing-features)
  - [1.3 Writing tests](#13-writing-tests)
- [2. General development guidelines](#2-general-development-guidelines)
  - [2.1 Code linting and formatting](#21-code-linting-and-formatting)
  - [2.2 Generating local documentation](#22-generating-local-documentation)
  - [2.3 Using the test harness](#23-using-the-test-harness)
  - [2.4 Modifying filesystem tree harnesses](#24-modifying-filesystem-tree-harnesses)
- [A1. Appendix: Project structure](#a1-appendix-project-structure)


<br>
<br>


## 1. Ways to Contribute


### 1.1 Bug reporting
If you encounter issues with `fs-more`, we encourage you to open an issue.
When doing so, please include as much context as you can, ideally with clear steps to reproduce the bug.


### 1.2 Developing features
Before developing new features or improving existing ones, 
please reach out first by creating a feature request issue in this repository. 
This way, other contributors can voice ideas and any potential concerns.

For new feature PRs, we encourage you to write tests as well (so someone else doesn't have to 😉).


### 1.3 Writing tests
New tests covering previously-untested code and edge cases are always welcome! 
Submit an issue describing what isn't well tested or submit a PR with a fresh batch of tests to review and merge.



<br>


## 2. General development guidelines

### 2.1 Code linting and formatting
To catch a large set of potential problems and unusual coding patterns, 
we use [clippy](https://github.com/rust-lang/rust-clippy) instead of `cargo check`.

As far as code formatting goes, 
we use nightly [rustfmt](https://github.com/rust-lang/rustfmt) with some rule overrides 
(see `rustfmt.toml` in the root of the repository).

Committed code should always be free of errors, ideally free of clippy warnings, 
and must be formatted with `rustfmt`.
If a specific `clippy` rule or `rustfmt`'s formatting doesn't make sense in a certain 
chunk of code, you *can* add an ignore for it (`#[allow(...)]` / `#[rustfmt::skip]`), 
*but do so sparingly*.


<details>
<summary>💡 Setup for Visual Studio Code (with <code>rust-analyzer</code>)</summary>
<br>

> This configuration requires [`rust-analyzer`](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) 
> to be installed and enabled in Visual Studio Code.

If you're using Visual Studio Code, you can use something akin to the configuration below to 
enable `clippy` and `rustfmt` as described above. Add these entries into your project-local `.vscode/settings.json`,
creating the file if necessary:

```json5
{
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer",
        "editor.formatOnSave": true
    },
    "rust-analyzer.check.overrideCommand": [
        "cargo",
        "clippy",
        "--workspace",
        "--message-format=json",
        "--all-targets",
    ],
    "rust-analyzer.rustfmt.extraArgs": [
        "+nightly"
    ],
    "rust-analyzer.cargo.features": "all"
}
```

Alongside `rust-analyzer` and this configuration, I'd suggest the following extensions:
- **(highly recommended)** [EditorConfig](https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig),
- *(good-to-have)* [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml), and
- *(optional; highlights comments)* [Better Comments](https://marketplace.visualstudio.com/items?itemName=aaron-bond.better-comments).

For Better Comments, the following configuration might be of use — add this to `.vscode/settings.json` after installing the extension:

```json5
{
    // ...
    "better-comments.tags": [
        {
            "tag": "todo",
            "color": "#77BAF5",
            "strikethrough": false,
            "underline": false,
            "backgroundColor": "transparent",
            "bold": false,
            "italic": false
        },
        {
            "tag": "debugonly",
            "color": "#c4b1e5",
            "strikethrough": false,
            "underline": false,
            "backgroundColor": "transparent",
            "bold": false,
            "italic": false
        },
        {
            "tag": "deprecated",
            "color": "#F5A867",
            "strikethrough": false,
            "underline": false,
            "backgroundColor": "transparent",
            "bold": false,
            "italic": false
        },
        {
            "tag": "fixme",
            "color": "#f26344",
            "strikethrough": false,
            "underline": false,
            "backgroundColor": "transparent",
            "bold": false,
            "italic": false
        },
    ]
    // ...
}
```

</details>



### 2.2 Generating local documentation
> This requires [`cargo-make`](https://github.com/sagiegurari/cargo-make) 
> and [`cargo-watch`](https://github.com/watchexec/cargo-watch)
> to be installed on the system.

To build documentation for the local development version of `fs-more`, run:

```bash
cargo make doc
```

This will build the documentation and open it in your default browser, 
as well as automatically recompile it when you make changes.


### 2.3 Using the test harness
To aid in writing tests, the `fs_more_test_harness` crate is available inside `subcrates/test-harness`.
It provides:
- the `detect_case_sensitivity_for_temp_dir` function that detects case-sensitivity of the filesystem, and
- a set of filesystem trees that can be used to initialize the same directory tree every time, 
  inspect it as a strongly-typed tree, 
  perform assertions on files and directories inside it, 
  as well as snapshot file data, etc.

There is also a `TestResult` type that can be used as a return value in tests to allow for 
e.g. ?-returning `std::io::Error`s, but is rarely useful.


Currently, the following filesystem tree harnesses are available:
- `DeepTree` (`subcrates/test-harness/src/trees/generated/deep.rs`),
- `SimpleTree` (`subcrates/test-harness/src/trees/generated/simple.rs`),
- `EmptyTree` (`subcrates/test-harness/src/trees/generated/empty.rs`),
- `SymlinkedTree` (`subcrates/test-harness/src/trees/generated/symlinked.rs`), and
- `BrokenSymlinksTree` (`subcrates/test-harness/src/trees/generated/broken_symlinks.rs`).

All of them essentially represent a single consistent directory tree,
but to showcase how they work, this section will focus on one of them - `DeepTree`. 

> If you're looking for more context about how this harness is constructed
> and generated, take a look at the next chapter.

<br>

To initialize a filesystem tree, call its `initialize` method, like so:
```rust,no_run
use fs_more_test_harness::prelude::*;
use fs_more_test_harness::trees::structures::deep::DeepTree;

let deep_harness = DeepTree::initialize();
```

Once initialized, `deep_harness.as_path()` will return the path 
to the temporary directory the harness is initialized at.

What we have at this point is a fully initialized directory tree on disk that
we can interact with in our integration tests. But the usability of this
harness is not just in initializing the same tree every time - it also
allows us to traverse the tree as a strongly-typed structure!

For example, the `DeepTree` we're using in this example has 
the following directory structure on disk:
```md
.
|-> a.bin (binary data, 32 KiB)
|-- foo
|   |-- bar
|   |   |-- hello
|   |   |   |-- world
|   |   |   |   |-> d.bin (binary data, 256 KiB)
|   |   |-> c.bin (binary data, 128 KiB)
|   |-> b.bin (binary data, 64 KiB)
```


As we can see, the root of the tree contains the `a.bin` file and 
the `foo` directory. The `foo` directory contains `b.bin` and `bar`, 
and so on. Where the harness shines is in the tree structure is provides:
`a.bin` is available as the `a_bin` field on `DeepTree`, and so is `foo`!

Here's a few examples on how we can "traverse" and inspect 
this tree in our tests:
```rust,no_run
use std::path::Path;

use fs_more_test_harness::prelude::*;
use fs_more_test_harness::trees::structures::deep::DeepTree;


let deep_harness = DeepTree::initialize();


deep_harness.assert_is_directory_and_not_empty();

assert_eq!(
    deep_harness.foo.bar.c_bin.as_path(),
    deep_harness.as_path().join("foo/bar/c.bin")
);

assert_eq!(
    deep_harness.foo.bar.c_bin.as_path_relative_to_harness_root(),
    Path::new("./foo/bar/c.bin")
);

deep_harness.foo.child_path("something/weird.txt").assert_not_exists();


// "snapshotting"
let captured_c_bin_state = deep_harness.foo.bar.c_bin.capture_with_content();

/* .. do something that must not change the file .. */

// .. and then verify that it truly didn't!
captured_c_bin_state.assert_unchanged();


deep_harness.foo.b_bin.assert_is_file_and_not_symlink();
deep_harness.foo.b_bin.assert_unchanged_from_initial_state();

// ... and so on 
//
// There are *many* many more assertions and other methods available:
// - symlinking,
// - asserting whether something is a file, directory, a symlink, etc.,
// - creating or deleting files and directories,
// - snapshotting,
// - obtaining file sizes,
// - and more.
```

> [!IMPORTANT]
> When in doubt, take a look at the documentation for each struct or field
> in the tree! For example, each root struct of the harness has documentation
> listing the entire filesystem structure it generates, as well as additional 
> context and the fields that are available directly on it.
>
> This is best and easiest with IDE mouse-over support.


There are many many more available methods than what is showcased here;
for more information, take a look at the traits available in
`src/assertable/trait.rs` and `src/tree_framework/traits.rs` inside the testing harness crate.


Finally, to clean up the harness, call its `destroy` method, like so:
```rust,no_run
use fs_more_test_harness::prelude::*;
use fs_more_test_harness::trees::structures::deep::DeepTree;

let deep_harness = DeepTree::initialize();

deep_harness.destroy();
```





### 2.4 Modifying filesystem tree harnesses
> This requires [`cargo-make`](https://github.com/sagiegurari/cargo-make)
> to be installed on the system.

Source files for individual filesystem trees, i.e. their structure, are written down
as JSON files inside the `subcrates/test-harness/trees` directory. Each JSON file
inside that directory defines a single filesystem tree. For the structure, take a look
at `subcrates/test-harness-generator/src/schema.rs` (see `FileSystemHarnessSchema` for the
root type).

In order to make those trees useful and strongly-typed, we need to generate Rust code that
initializes those trees and enables us to inspect the tree at runtime (well, test time). 
To do that, we can use the internal `subcrates/test-harness-generator` CLI.

To (re)generate the test harness trees from their JSON source files,
use the following command:

```bash
cargo make generate-test-harness-trees
```

This will collect all the JSON tree definitions inside `subcrates/test-harness/trees`
and generate Rust modules inside `subcrates/test-harness/src/trees/generated`.
Those are, in turn, exposed for usage in tests as `fs_more_test_harness::trees`.


<details>
<summary>💡 Setup for Visual Studio Code (JSON schema)</summary>
<br>


It's much easier to create and edit tree schemas when autocomplete is available.
As such, the CLI mentioned above can also generate a JSON schema that is used to define the trees.
To set up usage (autocompletion) for that schema in Visual Studio Code, add this to `.vscode/settings.json`:

```json5
{
    // ...
    "json.schemas": [
        {
            "fileMatch": [
                "subcrates/test-harness/trees/*.json"
            ],
            "url": "subcrates/test-harness/trees/_schema.json"
        }
    ]
    // ...
}
```

If the schema gets modified (see `subcrates/test-harness-generator/src/schema.rs`),
it will become necessary to regenerate the `_schema.json` file. To do that, run the following:

```bash
cargo make generate-test-harness-tree-schema
```

</details>


<br>


## A1. Appendix: Project structure
Here is a rough outline of the repository:
```md
|-- src
|   |> The root fs-more crate.
|
|-- subcrates
|   | |> Contains auxiliary crates, at the moment just 
|   |    the two crates related to the test harness.
|   |
|   |-- test-harness
|   |   |> Our test harness and useful reusable code 
|   |      for integration tests.
|   |
|   |-- test-harness-generator
|   |   |> Our test harness tree code generator CLI.
|   |      It generates code that acts as a specific filesystem tree
|   |      (that's our testing harness). The structure of each tree
|   |      is defined in `subcrates/test-harness/trees`.
|
|-- tests
|   |> Integration tests bunched together into a single `integration`
|      test binary (for performance). Individual integration tests are
|      sorted into the `directory` and `file` subdirectories
|      (and further into `copy`, `move`, etc.).
```
