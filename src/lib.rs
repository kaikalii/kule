#![allow(clippy::single_match)]

mod window;
pub use window::*;
mod error;
pub use error::*;
mod event;
pub use event::Event;
pub use event::*;

pub use vector2math::{f32::*, *};

#[cfg(test)]
#[test]
fn test() {
    Window::builder()
        .run(|window, event| {
            println!("{:?}", event);
            window
        })
        .unwrap();
}
