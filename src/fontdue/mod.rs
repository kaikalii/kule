//! Fontdue is a font parser, rasterizer, and layout tool.
//!
//! This is a #![no_std] crate, but still requires the alloc crate.

#![allow(dead_code)]
#![allow(clippy::transmute_int_to_float, clippy::transmute_float_to_int)]

extern crate alloc;

mod font;
/// Tools for laying out strings of text.
pub mod layout;
mod math;
mod platform;
mod raster;
mod unicode;

pub use font::*;

/// Alias for Result<T, &'static str>.
pub type FontResult<T> = Result<T, &'static str>;
