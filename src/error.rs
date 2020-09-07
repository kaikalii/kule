#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    DisplayCreation(#[from] glium::backend::glutin::DisplayCreationError),
}

pub type Result<T> = std::result::Result<T, Error>;
