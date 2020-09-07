#![allow(clippy::single_match)]

mod window;
pub use window::*;
mod error;
pub use error::*;
mod event;
pub use event::Event;
pub use event::*;
mod draw;
pub use draw::*;
mod color;
pub use color::*;

pub use vector2math::{f32::*, *};

#[cfg(test)]
#[test]
fn test() {
    Window::builder()
        .update(|_, window| {
            // println!("{:?}", event);
            window
        })
        .draw(|draw, window| {
            println!("{}", window.state.keys.get(Key::Space));
            draw.clear(Col::black());
            draw.rectangle([1.0, 0.0, 0.0, 1.0], Rect::new([0.0; 2], [0.1; 2]));
            draw.rectangle([0.0, 1.0, 0.0, 1.0], Rect::new([-0.1; 2], [0.1; 2]));
        })
        .run()
        .unwrap();
}
