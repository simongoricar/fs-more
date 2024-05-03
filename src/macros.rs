/// Imports (`use`s) `fs`.
///
/// If the user enables the `fs-err` feature,
/// this will import [`fs_err as fs`](https://docs.rs/fs-err).
/// If not, this will simply import the usual [`std::fs`].
///
/// Expands to
/// ```no_run
/// #[cfg(not(feature = "fs-err"))]
/// use std::fs;
///
/// #[cfg(feature = "fs-err")]
/// use fs_err as fs;
/// ```
#[macro_export]
macro_rules! use_enabled_fs_module {
    () => {
        #[cfg(not(feature = "fs-err"))]
        use std::fs;

        #[cfg(feature = "fs-err")]
        use fs_err as fs;
    };
}
