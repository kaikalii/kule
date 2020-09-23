use std::error::Error;

/// A kule error type
#[derive(Debug, thiserror::Error)]
pub enum KuleError {
    /// App error
    #[error("{0}")]
    App(Box<dyn Error>),
    /// Generic static error
    #[error("{0}")]
    Static(&'static str),
    /// IO error
    #[error("{0}")]
    IO(#[from] std::io::Error),
    /// Error creating the display
    #[error("{0}")]
    DisplayCreation(#[from] glium::backend::glutin::DisplayCreationError),
    /// Bad window icon data
    #[error("{0}")]
    BadIcon(#[from] glium::glutin::window::BadIcon),
    #[cfg(feature = "sound")]
    /// Audio decode error
    #[error("{0}")]
    AudioDecode(#[from] rodio::decoder::DecoderError),
}

impl KuleError {
    /// Create a new app error
    pub fn app<E>(error: E) -> Self
    where
        E: Error + 'static,
    {
        KuleError::App(Box::new(error))
    }
}

/// A kule result type
pub type KuleResult<T> = std::result::Result<T, KuleError>;
