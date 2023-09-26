/// Expands to a randomly generated (seeded) [`Vec<u8>`] (runtime-generated).
///
/// ## Examples
/// To generate 16 KiB of random data with the seed `37510903939111`:
///
/// ```rust
/// # use fs_more_test_harness::generate_seeded_binary_data;
/// let data: Vec<u8> = generate_seeded_binary_data!(
///     1024 * 16,
///     37510903939111
/// );
/// ```
#[macro_export]
macro_rules! generate_seeded_binary_data {
    ($file_size_bytes:expr, $seed:expr) => {{
        use rand::distributions::Standard;
        use rand::Rng;
        use rand_chacha::rand_core::SeedableRng;

        const SIZE_IN_BYTES: usize = $file_size_bytes;
        const SEED: u64 = $seed;

        let random_generator = rand_chacha::ChaCha20Rng::seed_from_u64(SEED);

        let mut __data: Vec<u8> = Vec::with_capacity(SIZE_IN_BYTES);

        __data.extend(
            random_generator
                .sample_iter::<u8, _>(Standard)
                .take(SIZE_IN_BYTES),
        );

        __data
    }};
}

/// Expands to a *lazily* randomly generated (seeded) [`Vec<u8>`] (still runtime-generated).
///
/// This is almost the same as [`generate_seeded_binary_data`], but wraps the
/// `Vec<u8>` in a [`once_cell::sync::Lazy`][../once_cell/sync/struct.Lazy.html],
/// meaning the contents are lazily generated on access.
///
/// ## Examples
/// To generate 16 KiB of random static data with the seed `37510903939111`:
///
/// ```rust
/// # use fs_more_test_harness::lazy_generate_seeded_binary_data;
/// # use once_cell::sync::Lazy;
/// static data: Lazy<Vec<u8>> = lazy_generate_seeded_binary_data!(
///     1024 * 16,
///     37510903939111
/// );
/// ```
#[macro_export]
macro_rules! lazy_generate_seeded_binary_data {
    ($file_size_bytes:expr, $seed:expr) => {
        once_cell::sync::Lazy::new(|| {
            use rand::distributions::Standard;
            use rand::Rng;
            use rand_chacha::rand_core::SeedableRng;

            const SIZE_IN_BYTES: usize = $file_size_bytes;
            const SEED: u64 = $seed;

            let random_generator = rand_chacha::ChaCha20Rng::seed_from_u64(SEED);

            let mut __data: Vec<u8> = Vec::with_capacity(SIZE_IN_BYTES);

            __data.extend(
                random_generator
                    .sample_iter::<u8, _>(Standard)
                    .take(SIZE_IN_BYTES),
            );

            __data
        })
    };
}
