#![allow(clippy::single_match)]

mod context;
pub use context::*;
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
    AppBuilder::<App>::new()
        .setup(|ctx| {
            ctx.load_font((), include_bytes!("../examples/firacode.ttf").as_ref())
                .unwrap();
            ctx.camera.zoom = [3.0; 2];
        })
        .update(|dt, ctx| {
            let wasd = ctx.tracker.key_diff2(Key::A, Key::D, Key::W, Key::S);
            let arrows = ctx
                .tracker
                .key_diff2(Key::Left, Key::Right, Key::Up, Key::Down);
            let plus_minus = ctx.tracker.key_diff(Key::Minus, Key::Equals);
            ctx.app.pos.add_assign(wasd.mul(100.0 * dt));
            ctx.camera.center.add_assign(arrows.mul(100.0 * dt));
            ctx.camera = ctx.camera.zoom_on(
                ctx.camera.zoom.mul(1.1f32.powf(plus_minus * dt * 10.0)),
                ctx.camera.coords_to_pos(ctx.app.pos),
            );
        })
        .draw(|draw, ctx| {
            draw.clear(Col::black());
            let rect = Rect::centered(ctx.app.pos, [40.0; 2]);
            let mut recter = draw.rectangle(Col::red(1.0), rect);
            recter.draw();
            recter.transform(rotate_about(1.0, ctx.app.pos)).draw();
            drop(recter);
            draw.circle([1.0, 0.5, 0.5], (ctx.app.pos, 15.0), 32)
                .border(Col::blue(1.0), 3.0);
            draw.line(Col::green(0.8), rect.bottom_left(), rect.top_right(), 1.0);
            draw.text(Col::white(), "Hello World! akfj sldkfj sldkfj slkdfjl sdkfjdlkjf slkdjf slkdfj sldkjflskd jflskdj flskfj", 100.0, ())
                .transform(zoom(0.4));
        })
        .run(App { pos: [0.0; 2] })
        .unwrap();
}
