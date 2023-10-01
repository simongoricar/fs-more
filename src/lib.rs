//! Convenient Rust file and directory operations.
//!
//! Features include: scanning directories, calculating file or directory sizes,
//! copying or moving files or directories,
//! copying or moving **with progress**, and more filesystem-oriented tools
//! that [`std::fs`] doesn't provide.
//!
//!
//! ### Attribution
//!
//! <details>
//! <summary>Inspired by <code>fs_extra</code></summary>
//!
//! `fs-more` isn't quite a fork, but has been inspired by
//! the [`fs_extra`](https://github.com/webdesus/fs_extra) library (thank you!), which is MIT-licensed:
//!
//! ```markdown
//! MIT License
//! Copyright (c) 2017 Denis Kurilenko
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ```
//!
//! </details>
//!

pub mod directory;
pub mod error;
pub mod file;
