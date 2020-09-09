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

pub use vector2math::{
    f32::*, Circle, FloatingScalar, FloatingVector2, Rectangle, Scalar, Transform, Vector2,
};

pub fn translate(offset: Vec2) -> impl Fn(Trans) -> Trans {
    move |trans| trans.translate(offset)
}

pub fn scale(ratios: Vec2) -> impl Fn(Trans) -> Trans {
    move |trans| trans.scale(ratios)
}

pub fn rotate(radians: f32) -> impl Fn(Trans) -> Trans {
    move |trans| trans.rotate(radians)
}

pub fn zoom(ratio: f32) -> impl Fn(Trans) -> Trans {
    move |trans| trans.zoom(ratio)
}

pub fn rotate_about(radians: f32, pivot: Vec2) -> impl Fn(Trans) -> Trans {
    move |trans| trans.rotate_about(radians, pivot)
}

#[cfg(test)]
#[test]
fn test() {
    struct App {
        pos: Vec2,
    }
    App::builder()
        .setup(|window| {
            window
                .load_font((), include_bytes!("../examples/firacode.ttf").as_ref())
                .unwrap()
        })
        .update(|dt, window| {
            let wasd = window.tracker.key_diff2(Key::A, Key::D, Key::W, Key::S);
            let arrows = window
                .tracker
                .key_diff2(Key::Left, Key::Right, Key::Up, Key::Down);
            let plus_minus = window.tracker.key_diff(Key::Minus, Key::Equals);
            window.app.pos.add_assign(wasd.mul(100.0 * dt));
            window.camera.center.add_assign(arrows.mul(100.0 * dt));
            window.camera = window.camera.zoom_on(
                window.camera.zoom.mul(1.1f32.powf(plus_minus * dt * 10.0)),
                window.camera.coords_to_pos(window.app.pos),
            );
        })
        .draw(|draw, window| {
            draw.clear(Col::black());
            let rect = Rect::centered(window.app.pos, [40.0; 2]);
            let mut recter = draw.rectangle(Col::red(1.0), rect);
            recter.draw();
            recter.transform(rotate_about(1.0, window.app.pos)).draw();
            drop(recter);
            draw.circle([1.0, 0.5, 0.5], Circ::new(window.app.pos, 15.0), 32);
            draw.line(Col::green(0.8), rect.bottom_left(), rect.top_right(), 5.0);
            draw.character(Col::white(), 'g', 40.0, ());
        })
        .run(App { pos: [0.0; 2] })
        .unwrap();
}
