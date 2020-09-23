#![allow(clippy::single_match)]
#![warn(missing_docs)]

/*!
A textureless 2d game engine

Kule is a game engine with a focus on rendering vector graphics. It has no support for textures or sprites.
This makes making games easier for the unartistic programmer, but restricts art style.
*/

mod app;
pub use app::*;
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
    f32::*, Circle, FloatingScalar, FloatingVector2, Pair, Rectangle, Scalar, Transform, Vector2,
};

#[cfg(test)]
mod test {
    use super::*;
    #[derive(Debug)]
    struct App {
        pos: Vec2,
        rot: f32,
    }
    type Recs = GenericResources<(), &'static str>;
    impl Kule for App {
        type Resources = Recs;
        fn setup(ctx: &mut Context<Recs>) -> Self {
            ctx.load_font((), include_bytes!("../examples/firacode.ttf").as_ref())
                .unwrap();
            ctx.camera.zoom = 4.0;
            App {
                pos: [0.0; 2],
                rot: 1.0,
            }
        }
        fn update(dt: f32, app: &mut Self, ctx: &mut Context<Recs>) {
            let wasd = ctx.tracker.key_diff_vector(Key::A, Key::D, Key::W, Key::S);
            let arrows = ctx
                .tracker
                .key_diff_vector(Key::Left, Key::Right, Key::Up, Key::Down);
            let plus_minus = ctx.tracker.key_diff_scalar(Key::Minus, Key::Equals);
            let qe = ctx.tracker.key_diff_scalar(Key::Q, Key::E);
            app.pos.add_assign(wasd.mul(100.0 * dt));
            app.rot += qe * dt;
            ctx.camera.center.add_assign(arrows.mul(100.0 * dt));
            ctx.camera = ctx.camera.zoom_by(1.1f32.powf(plus_minus * dt * 10.0));
        }
        fn event(event: Event, app: &mut Self, ctx: &mut Context<Recs>) {
            match event {
                Event::MouseButton {
                    button: MouseButton::Left,
                    state: ButtonState::Pressed,
                } => app.pos = ctx.mouse_coords(),
                Event::Scroll([_, y]) => {
                    let old_coords = ctx.mouse_coords();
                    ctx.camera = ctx.camera.zoom_by(1.1f32.powf(y));
                    let new_coords = ctx.mouse_coords();
                    ctx.camera.center.sub_assign(new_coords.sub(old_coords));
                }
                _ => {}
            }
        }
        fn draw<C>(draw: &mut Drawer<C, Recs>, app: &Self, ctx: &Context<Recs>)
        where
            C: Canvas,
        {
            draw.clear(Col::black());
            let rect = Rect::centered(app.pos, [40.0; 2]);
            let mut recter = draw.rectangle(Col::red(1.0), rect);
            let mut bordered = recter.border(Col::red(0.4), 5.0);
            bordered.draw();
            bordered
                .transform(|t| t.rotate_about(app.rot, app.pos))
                .draw();
            drop(bordered);
            drop(recter);
            draw.circle([1.0, 0.5, 0.5], (app.pos, 15.0), 32)
                .border([0.0, 0.0, 1.0], 3.0);
            draw.round_line(Col::green(0.8), (rect.bottom_left(), rect.top_right()), 5.0);
            draw.with_absolute_camera(|draw| {
                let font_size = 70.0;
                let text = "Wow, pretty good!";
                let text_width = draw.fonts.width(text, font_size);
                draw.line(Col::white(), [3.0, font_size, text_width, font_size], 1.0);
                draw.text(Col::white(), text, font_size)
                    .transform(|t| t.translate([0.0, font_size]));
                draw.circle([1.0, 0.0, 1.0, 0.3], (ctx.tracker.mouse_pos(), 5.0), 10);
            });
            draw.circle([1.0, 1.0, 0.0, 0.3], (ctx.mouse_coords(), 5.0), 10);
        }
        fn teardown(app: Self, _: &mut Context<Recs>) {
            println!("{:?}", app);
        }
    }

    #[test]
    fn test() {
        App::run().unwrap();
    }
}
