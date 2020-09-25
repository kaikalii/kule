use std::error::Error;

use crate::{Context, Kule};

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
    #[cfg(feature = "script")]
    /// A toml serialization error
    #[error("{0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[cfg(feature = "script")]
    /// A toml deserialization error
    #[error("{0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[cfg(feature = "script")]
    /// A lua error
    #[error("{0}")]
    Lua(#[from] mlua::Error),
    #[cfg(feature = "script")]
    /// A lua serialization error
    #[error("{0}")]
    LuaSerialization(#[from] crate::LuaSerializeError),
    #[cfg(feature = "script")]
    /// A scripting enevironment initialization error
    #[error("The scripting environment failed to initialize: {0}")]
    ScriptInitialization(String),
}

impl KuleError {
    /// Create a new app error
    pub fn app<E>(error: E) -> Self
    where
        E: Error + 'static,
    {
        KuleError::App(Box::new(error))
    }
    /// Handle the error using an app's error handling method
    pub fn handle<A>(self, app: &mut A, ctx: &mut Context<A::Resources>)
    where
        A: Kule,
    {
        A::handle_error(self, app, ctx)
    }
}

/// A kule result type
pub type KuleResult<T> = std::result::Result<T, KuleError>;

/// A possible error but no return type
pub type CanFail = KuleResult<()>;
