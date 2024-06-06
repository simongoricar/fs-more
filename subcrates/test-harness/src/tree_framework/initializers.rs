use std::{
    fs::OpenOptions,
    io::{prelude::Write, BufWriter},
    path::Path,
};

use rand::{
    distributions::Standard,
    prelude::{Rng, SeedableRng},
};

pub(crate) fn initialize_empty_file(file_path: &Path) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(file_path)
        .expect("failed to open file");

    file.flush().expect("failed to flush file");
}


pub(crate) fn initialize_file_with_string<S>(file_path: &Path, content: S)
where
    S: Into<String>,
{
    let mut buffered_file_writer = {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(file_path)
            .expect("failed to open file");

        BufWriter::new(file)
    };


    buffered_file_writer
        .write_all(content.into().as_bytes())
        .expect("failed to write string content to file");


    let mut file = buffered_file_writer
        .into_inner()
        .expect("failed to flush buffered writer");

    file.flush().expect("failed to flush file");
}

pub(crate) fn initialize_file_with_random_data(
    file_path: &Path,
    seed: u64,
    file_size_bytes: usize,
) {
    let random_generator = rand_chacha::ChaCha20Rng::seed_from_u64(seed);

    let mut random_data: Vec<u8> = Vec::with_capacity(file_size_bytes);
    random_data.extend(
        random_generator
            .sample_iter::<u8, _>(Standard)
            .take(file_size_bytes),
    );


    let mut buffered_file_writer = {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(file_path)
            .expect("failed to open file");

        BufWriter::new(file)
    };


    buffered_file_writer
        .write_all(&random_data)
        .expect("failed to write byte content to file");


    let mut file = buffered_file_writer
        .into_inner()
        .expect("failed to flush buffered writer");

    file.flush().expect("failed to flush file");
}
