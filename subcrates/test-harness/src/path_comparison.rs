use std::path::Path;

// TODO use these in tests so we don't have issues with mixed UNC paths

/// Returns `true` if the provided paths are the same
/// (ignoring UNC, if possible).
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
