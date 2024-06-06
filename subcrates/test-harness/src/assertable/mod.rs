use std::path::{Path, PathBuf};

pub mod dir_comparison;
pub mod file_capture;
pub mod file_comparison;
pub mod r#trait;

pub trait AsPath {
    fn as_path(&self) -> &Path;
}


impl<P> AsPath for P
where
    P: AsRef<Path>,
{
    fn as_path(&self) -> &Path {
        self.as_ref()
    }
}



pub trait WithSubPath {
    fn sub_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>;
}

impl<A> WithSubPath for A
where
    A: AsPath,
{
    fn sub_path<P>(&self, sub_path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.as_path().join(sub_path)
    }
}
