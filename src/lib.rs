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
    struct App {
        pos: Vec2,
    }
    App::builder()
        .event(|_, window| window)
        .update(|dt, window| Window {
            app: App {
                pos: window.app.pos.add(
                    [
                        window.tracker.keys.diff(Key::A, Key::D),
                        window.tracker.keys.diff(Key::S, Key::W),
                    ]
                    .mul(10.0 * dt),
                ),
            },
            camera: window
                .camera
                .map_center(|center| {
                    center.add(
                        [
                            window.tracker.keys.diff(Key::Left, Key::Right),
                            window.tracker.keys.diff(Key::Down, Key::Up),
                        ]
                        .mul(10.0 * dt),
                    )
                })
                .map_zoom(|zoom| {
                    zoom * 1.1f32.powf(window.tracker.keys.diff(Key::Minus, Key::Equals) * dt)
                }),
            ..window
        })
        .draw(|draw, window| {
            draw.clear(Col::black());
            draw.rectangle(
                [0.0, 1.0, 0.0, 1.0],
                Rect::centered(window.app.pos, [40.0; 2]),
            );
        })
        .run(App { pos: [0.0; 2] })
        .unwrap();
}
