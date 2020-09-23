use kule::*;

// An example app
struct App {
    pos: Vec2,
    rotation: f32,
}

// The `Kule` trait defines app behavior
impl Kule for App {
    type Resources = ();
    // The `build` method lets use define our app context
    fn build() -> KuleResult<ContextBuilder> {
        Ok(ContextBuilder::new()
            .title("Kule Guided Tour")
            .size([800.0; 2]))
    }
    // The `setup` method lets us initialize the app
    fn setup(ctx: &mut Context<Self::Resources>) -> KuleResult<Self> {
        println!("Setting up...");
        // Lets load a font
        ctx.load_only_font(include_bytes!("firacode.ttf"))?;

        Ok(App {
            pos: [0.0; 2],
            rotation: 0.0,
        })
    }
    // The `update` method is called often and lets us update app state absed on time
    fn update(dt: f32, app: &mut Self, ctx: &mut Context<Self::Resources>) {
        // The tracker tracks various input state
        let tracker = &ctx.tracker;
        // We can easily create a control vector to control the positon with WASD
        let wasd = tracker.key_diff_vector(Key::A, Key::D, Key::W, Key::S);
        const SPEED: f32 = 300.0;
        app.pos.add_assign(wasd.mul(SPEED * dt));
        // Lets control the rotation with Q and E
        let qe = tracker.key_diff_scalar(Key::Q, Key::E);
        app.rotation += qe * dt;
        // We can control the camera position with the arrow keys
        let arrows = tracker.key_diff_vector(Key::Left, Key::Right, Key::Up, Key::Down);
        ctx.camera.center.add_assign(arrows.mul(SPEED * dt));
        // Lets control the camera's zoom with + and -
        let plus_minus = tracker.key_diff_scalar(Key::Minus, Key::Equals);
        ctx.camera.zoom *= 1.1f32.powf(plus_minus * dt * 10.0);
    }
    // The `event` method lets us handle events
    fn event(event: Event, app: &mut Self, ctx: &mut Context<Self::Resources>) {
        // Check for a left-click
        if let Event::MouseButton {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        } = event
        {
            // `Context::mouse_coords` gets the coordinates of the mouse in world space
            app.pos = ctx.mouse_coords();
        }
    }
    // The `draw` method lets us draw to a generic canvas
    fn draw<C>(draw: &mut Drawer<C, Self::Resources>, app: &Self, ctx: &Context<Self::Resources>)
    where
        C: Canvas,
    {
        // Clear the background with black
        draw.clear(Col::black());
        // Drawing shapes is easy
        draw.line(Col::gray(0.5), [app.pos, [0.0; 2]], 5.0);
        draw.circle(Col::blue(1.0), ([0.0; 2], 25.0), 16)
            .border([0.15, 0.15, 1.0], 5.0);
        // Here we draw a square representing a player
        draw.rectangle(Col::red(1.0), Rect::square_centered(app.pos, 50.0))
            .transform(|t| t.rotate_about(app.rotation, app.pos));
        // `Drawer::with_absolute_camera` lets us draw things like UI
        draw.with_absolute_camera(|draw| {
            const PX: f32 = 50.0;
            // Text is easy to draw
            // We can create the draw instruction first
            let mut text = draw.text(Col::white(), "Graphics!", PX);
            // Then modify it and draw it multiple times
            text.translate([0.0, PX]); // Translate the text down and draw it
            text.translate([0.0, PX * 2.0]).color(Col::red(1.0)); // Translate the text down, color it, and draw it
            text.translate([0.0, PX * 3.0]).color(Col::yellow(1.0)); // etc...
            text.translate([0.0, PX * 4.0]).color(Col::green(1.0));
            text.translate([0.0, PX * 5.0]).color(Col::blue(1.0));
            drop(text);

            // We can draw text with different resolutions
            // Resolutions below 60 should probably be avoided, but this depends on the font
            let glyph_size = GlyphSize::new(80.0).resolution(60);
            // Lets use this different text resolution to draw an fps counter
            let fps_offset = [0.0, draw.camera.window_size().y() - 20.0];
            draw.text(
                Col::white(),
                &format!("fps: {}", ctx.tracker.fps().round()),
                glyph_size,
            )
            .transform(|t| t.translate(fps_offset));
        });
    }
    // The `teardown` method lets us call some code when the window is closed
    fn teardown(app: Self, _ctx: &mut Context<Self::Resources>) {
        println!("Quit at {:?}.", app.pos);
    }
}

fn main() {
    App::run().unwrap();
}
