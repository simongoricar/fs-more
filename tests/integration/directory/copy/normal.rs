use fs_more::{
    directory::{
        BrokenSymlinkBehaviour,
        CollidingSubDirectoryBehaviour,
        DestinationDirectoryRule,
        DirectoryCopyDepthLimit,
        DirectoryCopyOptions,
        DirectoryScanDepthLimit,
        DirectoryScanOptions,
        SymlinkBehaviour,
    },
    error::{
        CopyDirectoryError,
        CopyDirectoryPreparationError,
        DestinationDirectoryPathValidationError,
        DirectoryExecutionPlanError,
    },
    file::{CollidingFileBehaviour, FileCopyOptions},
};
use fs_more_test_harness::{
    collect_directory_statistics_via_scan,
    collect_directory_statistics_via_scan_with_options,
    prelude::*,
    trees::structures::{
        broken_symlinks::BrokenSymlinksTree,
        deep::DeepTree,
        empty::EmptyTree,
        simple::SimpleTree,
        symlinked::SymlinkedTree,
    },
};



#[test]
fn copy_directory_creates_an_identical_copy() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let deep_harness_stats = collect_directory_statistics_via_scan(deep_harness.as_path()).unwrap();


    let finished_copy = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            ..Default::default()
        },
    )
    .unwrap();


    assert_eq!(finished_copy.total_bytes_copied, deep_harness_stats.total_bytes);

    assert_eq!(finished_copy.files_copied, deep_harness_stats.total_files);

    assert_eq!(
        finished_copy.directories_created,
        deep_harness_stats.total_directories
    );

    empty_harness.assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());



    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_respects_copy_depth_limit() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    const MAXIMUM_SCAN_DEPTH: DirectoryScanDepthLimit =
        DirectoryScanDepthLimit::Limited { maximum_depth: 2 };

    const MAXIMUM_COPY_DEPTH: DirectoryCopyDepthLimit =
        DirectoryCopyDepthLimit::Limited { maximum_depth: 2 };


    let deep_harness_stats = collect_directory_statistics_via_scan_with_options(
        deep_harness.as_path(),
        DirectoryScanOptions {
            yield_base_directory: false,
            maximum_scan_depth: MAXIMUM_SCAN_DEPTH,
            ..Default::default()
        },
    )
    .unwrap();


    let finished_copy = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowEmpty,
            copy_depth_limit: MAXIMUM_COPY_DEPTH,
            ..Default::default()
        },
    )
    .unwrap();


    assert_eq!(finished_copy.total_bytes_copied, deep_harness_stats.total_bytes);

    assert_eq!(finished_copy.files_copied, deep_harness_stats.total_files);

    assert_eq!(
        finished_copy.directories_created,
        deep_harness_stats.total_directories
    );


    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_errors_when_source_and_destination_are_the_same() {
    let deep_harness = DeepTree::initialize();


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        deep_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.as_path())
    );


    deep_harness.destroy();
}



#[test]
fn copy_directory_errors_when_destination_is_inside_source_path() {
    let deep_harness = DeepTree::initialize();


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        deep_harness.foo.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.foo.as_path())
    );


    deep_harness.destroy();
}



#[test]
fn copy_directory_errors_when_destination_directory_already_exists_and_rule_is_disallow_existing() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::DisallowExisting,
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::AlreadyExists { path, destination_directory_rule }
            )
        )
        if paths_equal_no_unc(&path, empty_harness.as_path())
            && destination_directory_rule == DestinationDirectoryRule::DisallowExisting
    );


    deep_harness.assert_is_directory_and_not_empty();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_errors_when_destination_file_collides_and_its_behaviour_is_abort() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // Manually copy one deep harness file over to the empty harness.
    // Afterwards, we'll copy the entire tree over with [`ExistingFileBehaviour::Abort`],
    // meaning the call should error (because there is already a colliding file in the destination).
    let colliding_file_path = {
        empty_harness.assert_is_directory_and_empty();

        let a_bin_relative_path = deep_harness.a_bin.as_path_relative_to_harness_root();
        let empty_harness_colliding_file_path = empty_harness.child_path(a_bin_relative_path);

        empty_harness_colliding_file_path.assert_not_exists();


        fs_more::file::copy_file(
            deep_harness.foo.bar.c_bin.as_path(),
            &empty_harness_colliding_file_path,
            FileCopyOptions {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
            },
        )
        .unwrap();


        empty_harness_colliding_file_path.assert_is_file_and_not_symlink();

        deep_harness
            .foo
            .bar
            .c_bin
            .assert_initial_state_matches_other_file(&empty_harness_colliding_file_path);

        empty_harness.assert_is_directory_and_not_empty();

        empty_harness_colliding_file_path
    };


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        ) if paths_equal_no_unc(&path, &colliding_file_path)
    );


    colliding_file_path.assert_is_file_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
}




#[test]
fn copy_directory_errors_when_destination_subdirectory_collides_and_its_behaviour_is_abort() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // Manually copy one deep harness directory over to the empty harness.
    // Afterwards, we'll copy the entire tree over with [`ExistingSubDirectoryBehaviour::Abort`],
    // meaning the call should error (because there is already an existing colliding directory in the destination).
    let colliding_directory_path = {
        empty_harness.assert_is_directory_and_empty();


        let colliding_directory_name = deep_harness.foo.as_path().file_name().unwrap();
        let empty_harness_colliding_directory_path =
            empty_harness.child_path(colliding_directory_name);


        empty_harness_colliding_directory_path.assert_not_exists_and_create_empty_directory();
        empty_harness.assert_is_directory_and_not_empty();

        empty_harness_colliding_directory_path
    };


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        ) if paths_equal_no_unc(&path, &colliding_directory_path)
    );


    colliding_directory_path.assert_is_empty_directory_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_creates_destination_directory_if_missing() {
    let source_tree = SimpleTree::initialize();
    let destination_tree = EmptyTree::initialize();

    let destination_path = destination_tree.child_path("destination/hello/world");
    destination_path.assert_not_exists();


    fs_more::directory::copy_directory(
        source_tree.as_path(),
        &destination_path,
        DirectoryCopyOptions::default(),
    )
    .unwrap();


    destination_path
        .assert_is_directory_and_fully_matches_secondary_directory(source_tree.as_path());


    source_tree.destroy();
    destination_tree.destroy();
}




#[test]
fn copy_directory_respects_copy_depth_limit_if_source_contains_dir_symlink_and_behaviour_is_set_to_follow(
) {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // This block creates a symbolic link inside the deep tree named
    // `./symlink-to-bar`, which leads to `./foo/bar`.
    //
    // `c_bin_under_symlink_to_bar_in_destination` and `hello_dir_under_symlink_to_bar_in_destination`
    // lead to `./symlink-to-bar/c.bin` and `./symlink-to-bar/hello`, respectively.
    // These two paths should exist after the copy as normal files, since the symlink behaviour is set to follow.
    //
    // `world_dir_under_symlink_to_bar_in_destination` leads to `./symlink-to-bar/hello/world`,
    // which, given that the copy depth will be 1, must not exist after the copy.
    let (
        symlink_to_bar_in_destination,
        c_bin_under_symlink_to_bar_in_destination,
        hello_dir_under_symlink_to_bar_in_destination,
        world_dir_under_symlink_to_bar_in_destination,
    ) = {
        let symlink_to_bar_in_source = deep_harness.child_path("symlink-to-bar");
        symlink_to_bar_in_source.assert_not_exists();
        symlink_to_bar_in_source.symlink_to_directory(deep_harness.foo.bar.as_path());


        let symlink_to_bar_in_destination = empty_harness.child_path("symlink-to-bar");
        symlink_to_bar_in_destination.assert_not_exists();


        let c_bin_under_symlink_to_bar_in_destination = symlink_to_bar_in_destination
            .join(deep_harness.foo.bar.c_bin.as_path().file_name().unwrap());
        c_bin_under_symlink_to_bar_in_destination.assert_not_exists();

        let hello_dir_under_symlink_to_bar_in_destination = symlink_to_bar_in_destination
            .join(deep_harness.foo.bar.hello.as_path().file_name().unwrap());
        hello_dir_under_symlink_to_bar_in_destination.assert_not_exists();

        let world_dir_under_symlink_to_bar_in_destination =
            hello_dir_under_symlink_to_bar_in_destination.join(
                deep_harness
                    .foo
                    .bar
                    .hello
                    .world
                    .as_path()
                    .file_name()
                    .unwrap(),
            );
        world_dir_under_symlink_to_bar_in_destination.assert_not_exists();


        (
            symlink_to_bar_in_destination,
            c_bin_under_symlink_to_bar_in_destination,
            hello_dir_under_symlink_to_bar_in_destination,
            world_dir_under_symlink_to_bar_in_destination,
        )
    };


    fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            copy_depth_limit: DirectoryCopyDepthLimit::Limited { maximum_depth: 1 },
            symlink_behaviour: SymlinkBehaviour::Follow,
            ..Default::default()
        },
    )
    .unwrap();


    symlink_to_bar_in_destination.assert_is_directory_and_not_empty();
    c_bin_under_symlink_to_bar_in_destination.assert_is_file_and_not_symlink();
    hello_dir_under_symlink_to_bar_in_destination.assert_is_directory_and_empty();
    world_dir_under_symlink_to_bar_in_destination.assert_not_exists();


    deep_harness
        .foo
        .bar
        .c_bin
        .assert_initial_state_matches_other_file(&c_bin_under_symlink_to_bar_in_destination);


    deep_harness.destroy();
    empty_harness.destroy();
}


#[test]
fn copy_directory_respects_copy_depth_limit_if_source_contains_symlinks_and_behaviour_is_set_to_keep(
) {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    // This block creates the following symbolic links inside the deep tree:
    // - `./symlink-to-bar`, which leads to `./foo/bar`, and
    // - `./foo/symlink-to-d.bin`, which leads to `./foo/bar/hello/world/d.bin`.
    // - `./foo/bar/symlink-to-b.bin`, which leads to `./foo/b.bin`.
    //
    // Given a copy depth of 1 and symlink behaviour set to "keep",
    // `./symlink-to-bar` and `./foo/symlink-to-d.bin` should exist
    // on the destination as symlinks, but `./foo/bar/symlink-to-b.bin` should not.
    // Additionally, `./symlink-to-bar` should resolve to a valid directory with
    // the same contents as in the source.
    let (
        symlink_to_bar_in_destination,
        symlink_to_d_bin_in_destination,
        symlink_to_b_bin_in_destination,
    ) = {
        let symlink_to_bar_in_source = deep_harness.child_path("symlink-to-bar");
        symlink_to_bar_in_source.assert_not_exists();
        symlink_to_bar_in_source.symlink_to_directory(deep_harness.foo.bar.as_path());

        let symlink_to_bar_in_destination = empty_harness.child_path("symlink-to-bar");
        symlink_to_bar_in_destination.assert_not_exists();


        let symlink_to_d_bin_in_source = deep_harness.foo.child_path("symlink-to-d.bin");
        symlink_to_d_bin_in_source.assert_not_exists();
        symlink_to_d_bin_in_source
            .symlink_to_file(deep_harness.foo.bar.hello.world.d_bin.as_path());

        let symlink_to_d_bin_in_destination = empty_harness.child_path(
            deep_harness
                .foo
                .as_path_relative_to_harness_root()
                .join("symlink-to-d.bin"),
        );
        symlink_to_d_bin_in_destination.assert_not_exists();


        let symlink_to_b_bin_in_source = deep_harness.foo.bar.child_path("symlink-to-b.bin");
        symlink_to_b_bin_in_source.assert_not_exists();
        symlink_to_b_bin_in_source.symlink_to_file(deep_harness.foo.b_bin.as_path());

        let symlink_to_b_bin_in_destination = empty_harness.child_path(
            deep_harness
                .foo
                .bar
                .as_path_relative_to_harness_root()
                .join("symlink-to-b.bin"),
        );
        symlink_to_b_bin_in_destination.assert_not_exists();


        (
            symlink_to_bar_in_destination,
            symlink_to_d_bin_in_destination,
            symlink_to_b_bin_in_destination,
        )
    };


    fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            copy_depth_limit: DirectoryCopyDepthLimit::Limited { maximum_depth: 1 },
            symlink_behaviour: SymlinkBehaviour::Keep,
            ..Default::default()
        },
    )
    .unwrap();


    let resolved_symlink_to_bar_in_destination = symlink_to_bar_in_destination
        .assert_is_valid_symlink_to_directory_and_resolve_destination();
    resolved_symlink_to_bar_in_destination
        .assert_is_directory_and_fully_matches_secondary_directory_with_options(
            deep_harness.foo.bar.as_path(),
            true,
        );


    let resolved_symlink_to_d_bin_in_destination =
        symlink_to_d_bin_in_destination.assert_is_valid_symlink_to_file_and_resolve_destination();
    deep_harness
        .foo
        .bar
        .hello
        .world
        .d_bin
        .assert_initial_state_matches_other_file(resolved_symlink_to_d_bin_in_destination);

    symlink_to_b_bin_in_destination.assert_not_exists();


    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_preemptively_checks_for_directory_collisions() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let remapped_colliding_directory_path = {
        empty_harness.assert_is_directory_and_empty();

        let relative_path_to_bar = deep_harness.foo.as_path_relative_to_harness_root();

        let remapped_path = empty_harness.child_path(relative_path_to_bar);
        remapped_path.assert_not_exists_and_create_empty_directory();

        empty_harness.assert_is_directory_and_not_empty();

        remapped_path
    };


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Abort,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        )
        if paths_equal_no_unc(&path, &remapped_colliding_directory_path)
    );


    remapped_colliding_directory_path.assert_is_empty_directory_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_preemptively_checks_for_file_collisions() {
    let deep_harness = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    let remapped_colliding_file_path = {
        empty_harness.assert_is_directory_and_empty();

        let relative_path_to_a_bin = deep_harness.a_bin.as_path_relative_to_harness_root();

        let remapped_path = empty_harness.child_path(relative_path_to_a_bin);

        fs_more::file::copy_file(
            deep_harness.a_bin.as_path(),
            &remapped_path,
            FileCopyOptions {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
            },
        )
        .unwrap();

        empty_harness.assert_is_directory_and_not_empty();
        remapped_path.assert_is_file_and_not_symlink();

        remapped_path
    };


    let copy_result = fs_more::directory::copy_directory(
        deep_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Abort,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::CopyPlanningError(
                DirectoryExecutionPlanError::DestinationItemAlreadyExists { path }
            )
        ) if paths_equal_no_unc(&path, &remapped_colliding_file_path)
    );



    remapped_colliding_file_path.assert_is_file_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    empty_harness.destroy();
}



/// Tests fs_more behaviour when copying a "symlink to directory A" to "A".
/// This should return an error, regardless of overwriting configuration.
#[test]
fn copy_directory_errors_when_source_is_symlink_to_destination() {
    let deep_harness = DeepTree::initialize();
    let deep_harness_untouched = DeepTree::initialize();
    let empty_harness = EmptyTree::initialize();


    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());


    let symlink_to_deep_harnesss_path = {
        let symlink_path = empty_harness.child_path("directory-symlink");
        symlink_path.assert_not_exists();

        symlink_path.symlink_to_directory(deep_harness.as_path());

        empty_harness.assert_is_directory_and_not_empty();

        symlink_path
    };


    let copy_result = fs_more::directory::copy_directory(
        symlink_to_deep_harnesss_path.as_path(),
        deep_harness.as_path(),
        DirectoryCopyOptions {
            destination_directory_rule: DestinationDirectoryRule::AllowNonEmpty {
                colliding_file_behaviour: CollidingFileBehaviour::Overwrite,
                colliding_subdirectory_behaviour: CollidingSubDirectoryBehaviour::Continue,
            },
            ..Default::default()
        },
    );


    assert_matches!(
        copy_result.unwrap_err(),
        CopyDirectoryError::PreparationError(
            CopyDirectoryPreparationError::DestinationDirectoryValidationError(
                DestinationDirectoryPathValidationError::DescendantOfSourceDirectory { destination_directory_path, source_directory_path }
            )
        )
        if paths_equal_no_unc(&source_directory_path, deep_harness.as_path())
            && paths_equal_no_unc(&destination_directory_path, deep_harness.as_path())
    );


    deep_harness_untouched
        .assert_is_directory_and_fully_matches_secondary_directory(deep_harness.as_path());


    symlink_to_deep_harnesss_path.assert_is_symlink_and_remove();
    empty_harness.assert_is_directory_and_empty();


    deep_harness.destroy();
    deep_harness_untouched.destroy();
    empty_harness.destroy();
}



#[test]
fn copy_directory_does_not_preserve_symlinks_when_behaviour_is_set_to_follow() {
    let symlinked_harness = SymlinkedTree::initialize();
    let empty_harness = EmptyTree::initialize();


    fs_more::directory::copy_directory(
        symlinked_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Follow,
            ..Default::default()
        },
    )
    .unwrap();


    // Ensure ./foo/symlink-to-d.bin on the copy destination is not a symlink,
    // and that its contents match the symlink destination file on the copy source.
    {
        let symlink_to_d_bin_path_on_destination = empty_harness.as_path().join(
            symlinked_harness
                .foo
                .symlink_to_d_bin
                .as_path_relative_to_harness_root(),
        );

        symlink_to_d_bin_path_on_destination.assert_is_file_and_not_symlink();

        let symlink_to_d_bin_on_destination_state =
            CapturedFileState::new_with_content_capture(&symlink_to_d_bin_path_on_destination);


        let symlink_to_d_bin_path_on_source = symlinked_harness.foo.symlink_to_d_bin.as_path();
        let resolved_symlink_to_d_bin_path_on_source = symlink_to_d_bin_path_on_source
            .assert_is_valid_symlink_to_file_and_resolve_destination();

        let resolved_symlink_to_d_bin_on_source_state =
            CapturedFileState::new_with_content_capture(resolved_symlink_to_d_bin_path_on_source);


        resolved_symlink_to_d_bin_on_source_state
            .assert_captured_states_equal(&symlink_to_d_bin_on_destination_state);
    }


    // Ensure ./foo/symlink-to-hello on the copy destination is not a symlink,
    // and that its contents match the symlink destination directory on the copy source.
    {
        let symlink_to_hello_path_on_destination = empty_harness.as_path().join(
            symlinked_harness
                .foo
                .symlink_to_hello
                .as_path_relative_to_harness_root(),
        );

        symlink_to_hello_path_on_destination.assert_is_directory_and_not_symlink();


        let symlink_to_hello_path_on_source = symlinked_harness.foo.symlink_to_hello.as_path();
        let resolved_symlink_to_hello_path_on_source = symlink_to_hello_path_on_source
            .assert_is_valid_symlink_to_directory_and_resolve_destination();


        resolved_symlink_to_hello_path_on_source
            .assert_is_directory_and_fully_matches_secondary_directory_with_options(
                symlink_to_hello_path_on_destination,
                true,
            );
    }


    symlinked_harness.destroy();
    empty_harness.destroy();
}




#[test]
#[cfg(unix)]
fn copy_directory_does_not_preserve_relative_symlinks_when_behaviour_is_set_to_follow_on_unix() {
    use std::path::Path;


    let source_tree = SimpleTree::initialize();
    let destination_tree = EmptyTree::initialize();


    let expected_non_symlink_path_in_destination_tree = {
        let relative_symlink_path = Path::new("./empty.txt");

        let symlink_path_in_source_tree = source_tree.child_path("symlink-to-empty.txt");
        symlink_path_in_source_tree.symlink_to_file(relative_symlink_path);

        let expected_non_symlink_path_in_destination_tree =
            destination_tree.child_path("symlink-to-empty.txt");
        expected_non_symlink_path_in_destination_tree.assert_not_exists();


        expected_non_symlink_path_in_destination_tree
    };



    fs_more::directory::copy_directory(
        source_tree.as_path(),
        destination_tree.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Follow,
            ..Default::default()
        },
    )
    .unwrap();


    expected_non_symlink_path_in_destination_tree.assert_is_file_and_not_symlink();
    source_tree
        .empty_txt
        .assert_initial_state_matches_other_file(expected_non_symlink_path_in_destination_tree);


    destination_tree.destroy();
    source_tree.destroy();
}




#[test]
fn copy_directory_preserves_symlinks_when_behaviour_is_set_to_keep() {
    let symlinked_harness = SymlinkedTree::initialize();
    let empty_harness = EmptyTree::initialize();


    fs_more::directory::copy_directory(
        symlinked_harness.as_path(),
        empty_harness.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Keep,
            ..Default::default()
        },
    )
    .unwrap();


    // Ensure ./foo/symlink-to-d.bin on the copy destination is still a symlink
    // and that it points to the correct file.
    {
        let destination_d_bin_path = empty_harness.as_path().join(
            symlinked_harness
                .foo
                .symlink_to_d_bin
                .as_path_relative_to_harness_root(),
        );

        let resolved_destination_d_bin_path =
            destination_d_bin_path.assert_is_valid_symlink_to_file_and_resolve_destination();

        let resolved_destination_d_bin_state =
            CapturedFileState::new_with_content_capture(resolved_destination_d_bin_path);


        let resolved_source_d_bin_path = symlinked_harness
            .foo
            .symlink_to_d_bin
            .assert_is_valid_symlink_to_file_and_resolve_destination();

        let resolved_source_d_bin_state =
            CapturedFileState::new_with_content_capture(resolved_source_d_bin_path);


        resolved_destination_d_bin_state.assert_captured_states_equal(&resolved_source_d_bin_state);
    }

    // Ensure ./foo/symlink-to-hello on the copy destination is still a symlink
    // and that it points to the correct directory.
    {
        let destination_symlink_to_hello_path = empty_harness.as_path().join(
            symlinked_harness
                .foo
                .symlink_to_hello
                .as_path_relative_to_harness_root(),
        );

        let resolved_destination_symlink_to_hello_path = destination_symlink_to_hello_path
            .assert_is_valid_symlink_to_directory_and_resolve_destination();

        let resolved_source_symlink_to_hello_path =
            symlinked_harness.foo.symlink_to_hello.as_path();



        resolved_source_symlink_to_hello_path
            .assert_is_directory_and_fully_matches_secondary_directory_with_options(
                resolved_destination_symlink_to_hello_path,
                true,
            );
    }


    symlinked_harness.destroy();
    empty_harness.destroy();
}




#[test]
#[cfg(unix)]
fn copy_directory_preserves_and_may_make_broken_relative_symlinks_when_behaviour_is_set_to_keep_on_unix(
) {
    use std::path::Path;


    let source_tree = SimpleTree::initialize();
    let destination_tree = EmptyTree::initialize();


    let relative_symlink_path = Path::new("../empty.txt");
    let expected_broken_symlink_path_in_destination_tree = {
        let symlink_path_in_source_tree = source_tree.yes.child_path("symlink-to-empty.txt");
        symlink_path_in_source_tree.symlink_to_file(relative_symlink_path);

        let expected_broken_symlink_path_in_destination_tree =
            destination_tree.as_path().join("symlink-to-empty.txt");
        expected_broken_symlink_path_in_destination_tree.assert_not_exists();


        expected_broken_symlink_path_in_destination_tree
    };


    fs_more::directory::copy_directory(
        source_tree.yes.as_path(),
        destination_tree.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Keep,
            ..Default::default()
        },
    )
    .unwrap();


    let symlink_destination = expected_broken_symlink_path_in_destination_tree
        .assert_is_any_broken_symlink_and_read_destination();

    assert_eq!(&symlink_destination, relative_symlink_path);


    destination_tree.destroy();
    source_tree.destroy();
}




#[test]
fn copy_directory_preserves_broken_symlinks_when_behaviour_is_set_to_preserve() {
    let broken_symlink_harness = BrokenSymlinksTree::initialize();
    let destination_harness = EmptyTree::initialize();


    fs_more::directory::copy_directory(
        broken_symlink_harness.as_path(),
        destination_harness.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Keep,
            ..Default::default()
        },
    )
    .unwrap();


    {
        let broken_symlink_path_in_destination = destination_harness.child_path(
            broken_symlink_harness
                .foo
                .broken_symlink_txt
                .as_path_relative_to_harness_root(),
        );

        broken_symlink_path_in_destination.assert_is_any_broken_symlink();
    }


    broken_symlink_harness.destroy();
    destination_harness.destroy();
}



#[test]
fn copy_directory_aborts_on_broken_symlink_when_behaviour_is_set_to_abort() {
    let broken_symlink_harness = BrokenSymlinksTree::initialize();
    let destination_harness = EmptyTree::initialize();


    let copy_error = fs_more::directory::copy_directory(
        broken_symlink_harness.as_path(),
        destination_harness.as_path(),
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Keep,
            broken_symlink_behaviour: BrokenSymlinkBehaviour::Abort,
            ..Default::default()
        },
    )
    .unwrap_err();


    assert_matches!(
        copy_error,
        CopyDirectoryError::PreparationError(CopyDirectoryPreparationError::CopyPlanningError(DirectoryExecutionPlanError::SymbolicLinkIsBroken { path }))
        if paths_equal_no_unc(&path, broken_symlink_harness.foo.broken_symlink_txt.as_path())
    );


    broken_symlink_harness.destroy();
    destination_harness.destroy();
}



#[test]
fn copy_directory_preserves_source_directory_symbolic_link_when_behaviour_set_to_keep() {
    let simple_tree = SimpleTree::initialize();
    let copy_source_tree = EmptyTree::initialize();
    let copy_destination_tree = EmptyTree::initialize();

    let (copy_source_path, copy_destination_path) = {
        let source_directory_symlink_path = copy_source_tree.child_path("symlink-to-simple");
        source_directory_symlink_path.assert_not_exists();
        source_directory_symlink_path.symlink_to_directory(simple_tree.as_path());

        let destination_directory_symlink_path =
            copy_destination_tree.child_path("symlink-to-simple");
        destination_directory_symlink_path.assert_not_exists();


        (source_directory_symlink_path, destination_directory_symlink_path)
    };


    let finished_copy = fs_more::directory::copy_directory(
        copy_source_path,
        &copy_destination_path,
        DirectoryCopyOptions {
            symlink_behaviour: SymlinkBehaviour::Keep,
            ..Default::default()
        },
    )
    .unwrap();


    assert_eq!(finished_copy.files_copied, 0);
    assert_eq!(finished_copy.directories_created, 0);
    assert_eq!(finished_copy.symlinks_created, 1);


    copy_destination_path
        .assert_is_valid_symlink_to_directory_and_destination_matches(simple_tree.as_path());



    copy_source_tree.destroy();
    copy_destination_tree.destroy();
    simple_tree.destroy();
}
