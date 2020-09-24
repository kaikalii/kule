#![allow(clippy::single_match)]
#![warn(missing_docs)]

/*!
A textureless 2d game engine

Kule is a game engine with a focus on rendering vector graphics. It has no support for textures or sprites.
This makes making games easier for the unartistic programmer, but restricts art style.

# Usage

Check out [the guided tour](https://github.com/kaikalii/kule/blob/master/examples/guided_tour.rs) for a simple introduction.

## The `Kule` trait

The [`Kule`](trait.Kule.html) trait is the main trait that defines app behavior. Once you have a type that
implements it, simply call [`Kule::run`](trait.Kule.htm#method.run) in your `main` function.

### The `Context` struct

The [`Context`](struct.Context.html) struct holds the state of the engine. This includes resource caches, the window
handle, an input tracker, and many other things. Take a look at its documentation for more info.

## Resources

The [`Resources`](trait.Resources.html) trait defines type used for resource caching. Cached resources include
glyph geometry, 2D meshes, and sound buffers. The id types defined by `Resources` let you refer to cached items.
[`GenericResources`](struct.GenericResources.html) makes it easy to construct your own resource type.

## The `Drawer` struct

The [`Drawer`](struct.Drawer.html) struct is used to render 2D geometry.
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
#[cfg(feature = "sound")]
mod sound;
#[cfg(feature = "sound")]
pub use sound::*;
#[cfg(feature = "script")]
mod script;
#[cfg(feature = "script")]
pub use script::*;

pub use vector2math::{
    f32::*, Circle, FloatingScalar, FloatingVector2, Rectangle, Scalar, Transform, Vector2,
};

#[cfg(test)]
mod test {
    use super::*;
    #[derive(Debug)]
    struct App {
        pos: Vec2,
        rot: f32,
    }
    type Recs = GenericResources<(), (), &'static str>;
    impl Kule for App {
        type Resources = Recs;
        fn setup(ctx: &mut Context<Recs>) -> KuleResult<Self> {
            ctx.load_font((), include_bytes!("../examples/firacode.ttf").as_ref())
                .unwrap();
            ctx.camera.zoom = 4.0;
            Ok(App {
                pos: [0.0; 2],
                rot: 1.0,
            })
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
                Event::Key {
                    key: Key::Space,
                    state: ButtonState::Pressed,
                    ..
                } => {
                    ctx.play_sound("examples/kick.ogg", app).unwrap();
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
            let mut bordered = recter.border(Col::red(0.4), 3.0);
            bordered.draw();
            bordered
                .transform(|t| t.rotate_about(app.rot, app.pos))
                .draw();
            drop(bordered);
            drop(recter);
            draw.circle([1.0, 0.5, 0.5], (app.pos, 15.0), 32)
                .border([0.0, 0.0, 1.0], 3.0);
            draw.round_line(Col::green(0.8), (rect.bottom_left(), rect.top_right()), 3.0);
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
        fn load_sound(
            sound_id: &'static str,
            _app: &Self,
        ) -> KuleResult<Option<crate::sound::SoundBuffer>> {
            Ok(Some(SoundBuffer::decode(std::fs::read(sound_id)?)?))
        }
    }

    #[test]
    fn test() {
        App::run().unwrap();
    }
}
