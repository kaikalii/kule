/// A kule error type
#[derive(Debug, thiserror::Error)]
pub enum KuleError {
    /// Error creating the display
    #[error("{0}")]
    DisplayCreation(#[from] glium::backend::glutin::DisplayCreationError),
    /// Generic static error
    #[error("{0}")]
    Static(&'static str),
    /// Bad window icon data
    #[error("{0}")]
    BadIcon(#[from] glium::glutin::window::BadIcon),
}

/// A kule result type
pub type KuleResult<T> = std::result::Result<T, KuleError>;
