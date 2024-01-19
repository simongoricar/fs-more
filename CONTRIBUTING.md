`fs-more` Contributing Guide
============================

<br>

## Table of Contents
- [Table of Contents](#table-of-contents)
- [1. Bug reporting](#1-bug-reporting)
- [2. Developing features](#2-developing-features)
- [3. Writing tests](#3-writing-tests)
- [4. General development guidelines](#4-general-development-guidelines)
  - [4.1 Code linting and formatting](#41-code-linting-and-formatting)
  - [4.2 Generating up-to-date local documentation](#42-generating-up-to-date-local-documentation)
- [A. Appendix](#a-appendix)
  - [A1. Project structure](#a1-project-structure)


<br>
<br>

So you're thinking about contributing to `fs-more`? Awesome! Here are a few tips to get you started.


## 1. Bug reporting
If you encounter issues with `fs-more`, you're encouraged to open an issue in this repository.
When doing so, please include as much context as you can, ideally with clear steps to reproduce the bug.


## 2. Developing features
Before developing new features or improving existing ones that you would like to contribute back to upstream, 
please reach out first by creating a feature request issue in this repository. 
This way, other contributors can voice ideas and any potential concerns.

For new feature PRs, you're encouraged to write tests as well (so someone else doesn't have to).


## 3. Writing tests
Tests covering untested code and edge cases are always welcome. 
Please submit an issue describing what isn't well tested or submit a PR with a fresh batch of tests to review and merge.


---

## 4. General development guidelines

### 4.1 Code linting and formatting
To catch a larger set of potential problems, we use [clippy](https://github.com/rust-lang/rust-clippy) instead of a normal `cargo check`.
As far as code formatting goes, we use nightly [rustfmt](https://github.com/rust-lang/rustfmt) with some rule overrides (see `rustfmt.toml`).

Commited code should always be free of errors, ideally free of warnings, and be formatted with `rustfmt`. 
If a specific `clippy` rule or `rustfmt`'s output doesn't make sense in a certain chunk of code, 
you can add an ignore for it (`#[allow(...)]` / `#[rustfmt::skip]`), but do so sparingly.


<details>
<summary>Setup for Visual Studio Code (with <code>rust-analyzer</code>)</summary>
<br>

> [!IMPORTANT]
> This configuration requires [`rust-analyzer`](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) 
> to be installed and enabled in Visual Studio Code.

If you're using Visual Studio Code, you can use something akin to the configuration below to 
enable `clippy` and `rustfmt` as described above. Add these entries into your project-local `.vscode/settings.json`,
creating the file if necessary:

```json
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

```json
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



### 4.2 Generating up-to-date local documentation
To build documentation for the local development version of `fs-more`, run:

```bash
cargo doc --workspace --open
```

This will build the documentation and open it in your default browser.


<details>
<summary>Setup for Visual Studio Code (with <code>cargo-watch</code>)</summary>
<br>

> [!IMPORTANT]
> This configuration requires [cargo-watch](https://github.com/watchexec/cargo-watch) 
> to be installed on your system.

If you're using Visual Studio Code, you can use something akin to this configuration to add a task for 
generating documentation. This goes into `.vscode/tasks.json` (create the file if necessary):


```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "build and open documentation",
            "type": "cargo",
            "group": "build",
            "command": "doc",
            "args": ["--workspace", "--open"],
            "hide": true
        },
        {
            "label": "build and watch documentation",
            "type": "cargo",
            "group": "build",
            "command": "watch",
            "args": ["-x", "doc --workspace --no-deps"],
            "hide": true
        },
        {
            "label": "documentation (build, open, then watch)",
            "group": "build",
            "dependsOn": [
                "build and open documentation",
                "build and watch documentation"
            ],
            "dependsOrder": "sequence",
            "isBackground": true,            
        }
    ]
}
```

Then, run the `documentation (build, open, then watch)` task by selecting it in the `Task: Run Build Task` action 
(*Ctrl+Shift+B* is a useful shortcut to remember). 
This will generate the documentation, open it in your browser, and keep updating it as you make changes to the code.

</details>



## A. Appendix
### A1. Project structure
Before contributing, I'd suggest familiarizing yourself with this repository. Here is a rough outline of the contents:
```markdown
|-- src
|   |> The root fs-more crate.
|
|-- test-harness
|   |> fs-more's test harness (using assert_fs) and reusable code for integration tests.
|
|-- test-harness-derive
|   |> Test harness's procedural macro for setting up test directories.
|      See `test-harness/src/trees` for some usage examples.
|
|-- tests
|   |> Integration tests.
```
