#[macro_export]
macro_rules! generate_seeded_binary_data {
    ($file_size_bytes:expr, $seed:expr) => {{
        const SIZE_IN_BYTES: usize = $file_size_bytes;
        const SEED: u64 = $seed;

        let random_generator = rand_chacha::ChaCha20Rng::seed_from_u64(SEED);

        let mut data: Vec<u8> = Vec::with_capacity(SIZE_IN_BYTES);

        data.extend(
            random_generator
                .sample_iter::<u8, _>(Standard)
                .take(SIZE_IN_BYTES),
        );

        data
    }};
}

#[macro_export]
macro_rules! lazy_generate_seeded_binary_data {
    ($file_size_bytes:expr, $seed:expr) => {
        once_cell::sync::Lazy::new(|| {
            const SIZE_IN_BYTES: usize = $file_size_bytes;
            const SEED: u64 = $seed;

            let random_generator = rand_chacha::ChaCha20Rng::seed_from_u64(SEED);

            let mut data: Vec<u8> = Vec::with_capacity(SIZE_IN_BYTES);

            data.extend(
                random_generator
                    .sample_iter::<u8, _>(Standard)
                    .take(SIZE_IN_BYTES),
            );

            data
        })
    };
}
