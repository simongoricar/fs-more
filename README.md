fs-more
=======

Convenient Rust file and directory operations.
Features include: scanning directories, calculating file or directory sizes, 
copying or moving files or directories, 
copying or moving **with progress**, and more filesystem-oriented tools
that `std::fs` doesn't provide.

---

## Project status
**This is a work-in-progress.** Many features, such as the `file` module, already exist and are mostly stable, 
but are still expected to change before `1.0`.
Certain features, such as copying file/directory permissions or some directory operations, are not here yet. 
Unit, doc and integration tests exist and cover most of the base functionality, but fringe cases might not be covered yet 
([contributions](https://github.com/DefaultSimon/fs-more/blob/master/CONTRIBUTING.md) are welcome). 
As such, use this library with reasonable caution and testing.


## Installation
**`fs-more` isn't on [crates.io](https://crates.io/) yet.** Until then you can add it into your project with:
```toml
fs-more = { git = "https://github.com/DefaultSimon/fs-more" }
```

or, preferably:

```toml
fs-more = { git = "https://github.com/DefaultSimon/fs-more", rev = "commit hash here" }
```


## Contributing and development
A contribution guide is available in [`CONTRIBUTING.md`](https://github.com/DefaultSimon/fs-more/blob/master/CONTRIBUTING.md).

### Documentation
As the documentation isn't available on `docs.rs` yet, you need to build `fs-more`'s documentation locally.
To build and open the project documentation for the entire repository, run:

```bash
cargo doc --workspace --open
```

This will build the documentation and open it in your default browser.

If you're using Visual Studio Code, you can use something akin to this configuration to add a task for 
automatic compiling of the documentation (this goes into `.vscode/tasks.json`):

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

Then, run the `documentation (build, open, then watch)` task. 
This will compile the documentation, open it in your browser, and keep compiling it as you make changes.

> Note: this configuration requires [cargo-watch](https://github.com/watchexec/cargo-watch) to be installed.


## Attribution
<details>
<summary>Inspired by <code>fs_extra</code></summary>

`fs-more` isn't quite a fork, but has been inspired by 
the [`fs_extra`](https://github.com/webdesus/fs_extra) library (thank you!), which is MIT-licensed:

```markdown
MIT License
Copyright (c) 2017 Denis Kurilenko

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
</details>
