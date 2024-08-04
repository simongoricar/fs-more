use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    path::{Path, PathBuf},
};


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


/// Asserts that all paths in the `scanned_paths` iterator
/// appear in the `expected_set_of_paths` iterator (order is ignored).
///
/// If a path is missing, this function panics with the details.
#[track_caller]
pub fn assert_path_list_fully_matches_set<S, SP, D, DP>(scanned_paths: S, expected_set_of_paths: D)
where
    S: IntoIterator<Item = SP>,
    SP: AsRef<Path>,
    D: IntoIterator<Item = DP>,
    DP: AsRef<Path>,
{
    let scanned_path_set: HashSet<PathBuf> = HashSet::from_iter(
        scanned_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );

    let expected_path_set: HashSet<PathBuf> = HashSet::from_iter(
        expected_set_of_paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf()),
    );


    for scanned_path in scanned_path_set.iter() {
        if !expected_path_set.contains(scanned_path.as_path()) {
            panic!(
                "path \"{}\" was scanned, but not present in expected paths:\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                scanned_path.display(),
                scanned_path_set,
                expected_path_set
            );
        }
    }

    for expected_path in expected_path_set.iter() {
        if !scanned_path_set.contains(expected_path.as_path()) {
            panic!(
                "path \"{}\" was expected, but not present in scanned paths:\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                expected_path.display(),
                scanned_path_set,
                expected_path_set
            );
        }
    }
}


#[track_caller]
pub fn assert_path_list_fully_matches_with_counted_ocucrrences<S, SP, D, DP>(
    scanned_paths: S,
    expected_set_of_paths: D,
) where
    S: IntoIterator<Item = SP>,
    SP: AsRef<Path>,
    D: IntoIterator<Item = (DP, usize)>,
    DP: AsRef<Path>,
{
    let mut scanned_paths_with_occurrences: HashMap<PathBuf, usize> = HashMap::new();

    for scanned_path in scanned_paths {
        match scanned_paths_with_occurrences.entry(scanned_path.as_ref().to_path_buf()) {
            Entry::Occupied(mut existing_path_counter) => *existing_path_counter.get_mut() += 1,
            Entry::Vacant(missing_path_counter) => {
                missing_path_counter.insert(1);
            }
        }
    }

    let expected_paths_with_occurrences: HashMap<PathBuf, usize> = HashMap::from_iter(
        expected_set_of_paths
            .into_iter()
            .map(|(path, occurrences)| (path.as_ref().to_path_buf(), occurrences)),
    );



    for (scanned_path, scanned_occurrences) in scanned_paths_with_occurrences.iter() {
        let Some(expected_occurrences) =
            expected_paths_with_occurrences.get(scanned_path.as_path())
        else {
            panic!(
                "path \"{}\" was scanned, but not present in expected paths:\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                scanned_path.display(),
                scanned_paths_with_occurrences,
                expected_paths_with_occurrences
            );
        };


        if scanned_occurrences != expected_occurrences {
            panic!(
                "path \"{}\" was present on both sides, \
                but not the same number of times ({} scanned, {} expected):\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                scanned_path.display(),
                scanned_occurrences,
                expected_occurrences,
                scanned_paths_with_occurrences,
                expected_paths_with_occurrences
            );
        }
    }

    for (expected_path, expected_occurrences) in expected_paths_with_occurrences.iter() {
        let Some(scanned_occurrences) = scanned_paths_with_occurrences.get(expected_path.as_path())
        else {
            panic!(
                "path \"{}\" was expected, but not present in scanned paths:\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                expected_path.display(),
                scanned_paths_with_occurrences,
                expected_paths_with_occurrences
            );
        };


        if scanned_occurrences != expected_occurrences {
            panic!(
                "path \"{}\" was present on both sides, \
                but not the same number of times ({} scanned, {} expected):\n\n\
                {:?}\n  \
                  (scanned) versus (expected)\n\
                {:?}\n",
                expected_path.display(),
                scanned_occurrences,
                expected_occurrences,
                scanned_paths_with_occurrences,
                expected_paths_with_occurrences
            );
        }
    }
}
