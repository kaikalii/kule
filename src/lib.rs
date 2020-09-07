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
mod font;
pub use font::*;

pub use vector2math::{f32::*, *};

#[cfg(test)]
#[test]
fn test() {
    struct App {
        pos: Vec2,
    }
    App::builder()
        .event(|_, window| window)
        .update(|dt, window| {
            let wasd = window.tracker.key_diff2(Key::A, Key::D, Key::S, Key::W);
            let arrows = window
                .tracker
                .key_diff2(Key::Left, Key::Right, Key::Down, Key::Up);
            let plus_minus = window.tracker.key_diff(Key::Minus, Key::Equals);
            let app = App {
                pos: window.app.pos.add(wasd.mul(100.0 * dt)),
            };
            let camera = window
                .camera
                .map_center(|center| center.add(arrows.mul(100.0 * dt)))
                .map_zoom_on(
                    |zoom| zoom.mul(1.1f32.powf(plus_minus * dt * 10.0)),
                    window.camera.coords_to_pos(window.app.pos),
                );
            Window {
                app,
                camera,
                ..window
            }
        })
        .draw(|draw, window| {
            draw.clear(Col::black());
            let rect = Rect::centered(window.app.pos, [40.0; 2]);
            draw.rectangle(Col::red(1.0), rect);
            draw.circle([1.0, 0.5, 0.5], Circ::new(window.app.pos, 15.0), 32);
            draw.line(Col::green(0.8), rect.bottom_left(), rect.top_right(), 5.0);
        })
        .run(App { pos: [200.0; 2] })
        .unwrap();
}
