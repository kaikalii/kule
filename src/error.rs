#[derive(Debug, thiserror::Error)]
pub enum KuleError {
    #[error("{0}")]
    DisplayCreation(#[from] glium::backend::glutin::DisplayCreationError),
    #[error("{0}")]
    SwapBuffers(#[from] glium::SwapBuffersError),
    #[error("{0}")]
    Static(&'static str),
    #[error("{0}")]
    BadIcon(#[from] glium::glutin::window::BadIcon),
}

pub type KuleResult<T> = std::result::Result<T, KuleError>;
