use std::path::Path;


/// Returns `true` if the provided paths are the same
/// (ignoring UNC, if possible).
///
/// Use this function to compare paths in tests, to avoid
/// inconsistent results when disabling the `dunce` feature on `fs-more`.
#[track_caller]
pub fn paths_equal_no_unc<A, B>(first_path: A, second_path: B) -> bool
where
    A: AsRef<Path>,
    B: AsRef<Path>,
{
    if first_path.as_ref().eq(second_path.as_ref()) {
        return true;
    }


    let simplified_first_path = dunce::simplified(first_path.as_ref());
    let simplified_second_path = dunce::simplified(second_path.as_ref());

    simplified_first_path.eq(simplified_second_path)
}
