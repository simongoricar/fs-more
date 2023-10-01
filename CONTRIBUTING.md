`fs-more` contributing guide
============================

So you're thinking about contributing to `fs-more`? Awesome! Here are a few tips.

---

## 1. Bug reporting
If you encounter issues with `fs-more`, you're encouraged to open an issue in this repository.
When doing so, please include as much context as you can, ideally with clear steps to reproduce the bug.


## 2. Developing features
Before developing new features or improving existing ones, please
reach out first by creating a feature request issue in this repository. 
This way, me and any other contributors can voice ideas and any potential concerns.

For new features, I'd encourage you to write tests as well, so someone else doesn't have to.

## 3. Writing tests
Tests covering untested code and edge cases are basically always welcome, so you're welcome to 
submit an issue describing what isn't well tested and/or submitting a PR with a fresh batch of tests.


---

## 3. General development guidelines

### 3.1 Code linting and formatting
To catch more potential problems, we use [clippy](https://github.com/rust-lang/rust-clippy) instead of a normal `check`.
As far as code formatting goes, we use nightly [rustfmt](https://github.com/rust-lang/rustfmt) with some rule overrides.

Commited code should be always free of errors and warnings and formatted with `rustfmt`. If a specific clippy rule or rustfmt's output
doesn't make sense in a certain piece of code, you can add an ignore for it (`#[allow(...)]` / `#[rustfmt::skip]`), but do so sparingly.

If you're using Visual Studio Code, you can use something akin to this configuration to enable `clippy` and `rustfmt` as described above 
(this goes into `.vscode/settings.json`):
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
    ]
}
```

Note: you'll need the [`rust-analyzer`](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension 
for this to work.



## Appendix
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
|
|-- tests
|   |> Integration tests.
```
