use std::{fmt::Debug, hash::Hash, marker::PhantomData, time::Instant};

use glium::{glutin::*, *};

#[cfg(feature = "sound")]
use crate::sound::{self, SoundBuffer};
use crate::{
    Camera, CanFail, Canvas, Context, ContextBuilder, Drawer, Event, FloatingScalar, KuleResult,
    StateTracker, Window,
};

/**
The primary trait that defines app behavior
*/
#[allow(unused_variables)]
pub trait Kule: Sized + 'static {
    /// The resources type
    type Resources: Resources;
    /// Build the context
    fn build() -> KuleResult<ContextBuilder> {
        Ok(ContextBuilder::default())
    }
    /// Build the app
    fn setup(ctx: &mut Context<Self::Resources>) -> KuleResult<Self>;
    /// Update function called often
    ///
    /// `dt` is the amount of time that has passed since the last update
    fn update(dt: f32, app: &mut Self, ctx: &mut Context<Self::Resources>) -> CanFail {
        Ok(())
    }
    /// Draw
    fn draw<C>(
        draw: &mut Drawer<C, Self::Resources>,
        app: &Self,
        ctx: &Context<Self::Resources>,
    ) -> CanFail
    where
        C: Canvas,
    {
        Ok(())
    }
    /// Handle events
    fn event(event: Event, app: &mut Self, ctx: &mut Context<Self::Resources>) -> CanFail {
        Ok(())
    }
    /// Called when the app is closed
    fn teardown(app: Self, ctx: &mut Context<Self::Resources>) {}
    #[cfg(feature = "sound")]
    /// Load a sound
    fn load_sound(
        sound_id: <Self::Resources as Resources>::SoundId,
        app: &Self,
    ) -> KuleResult<Option<SoundBuffer>> {
        Ok(None)
    }
    #[cfg(feature = "script")]
    /// Handle an error
    ///
    /// The default implementation simply panics
    fn handle_error(error: crate::KuleError, app: &mut Self, ctx: &mut Context<Self::Resources>) {
        panic!("{}", error)
    }
    /// Run the app and panic if setup fails
    fn run_or_panic() -> ! {
        Self::run().unwrap_or_else(|e| panic!("{}", e));
        std::process::exit(0)
    }
    /// Run the app
    ///
    /// This takes control of the current thread. If initial setup does not fail, then this
    /// function will never return.
    fn run() -> KuleResult<std::convert::Infallible> {
        let builder = Self::build()?;
        #[cfg(feature = "script")]
        let script_env = builder.script_env.clone();
        let ContextBuilder {
            title,
            size,
            icon,
            samples,
            automatic_close,
            update_frequency,
            ..
        } = builder;
        // Init audio
        #[cfg(feature = "sound")]
        let sink = sound::sink();
        // Build event loop and display
        #[cfg(not(test))]
        let event_loop = event_loop::EventLoop::new();
        #[cfg(test)]
        let event_loop = {
            #[cfg(unix)]
            use platform::unix::EventLoopExtUnix;
            #[cfg(windows)]
            use platform::windows::EventLoopExtWindows;
            event_loop::EventLoop::<()>::new_any_thread()
        };
        let wb = window::WindowBuilder::new()
            .with_title(title)
            .with_window_icon(icon)
            .with_inner_size(dpi::LogicalSize::new(size[0], size[1]));
        let cb = glutin::ContextBuilder::new()
            .with_multisampling(samples)
            .with_stencil_buffer(1);
        let display = Display::new(wb, cb, &event_loop)?;
        let window_size = display.gl_window().window().inner_size();
        let program = crate::default_shaders(&display);
        let mut ctx = Context {
            program,
            fonts: Default::default(),
            meshes: Default::default(),
            #[cfg(feature = "sound")]
            mixer: sound::Mixer::new(&sink),
            #[cfg(feature = "sound")]
            sounds: sound::Sounds::default(),
            tracker: StateTracker::default(),
            camera: Camera {
                center: [0.0; 2],
                zoom: 1.0,
                window_size: window_size.into(),
            },
            window: Window(display),
            #[cfg(feature = "script")]
            scripts: crate::Scripts::load(script_env),
            should_close: false,
            update_timer: Instant::now(),
            fps_timer: Instant::now(),
        };
        // Run app setup
        let mut app = Some(Self::setup(&mut ctx)?);
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            // Draw
            if let event::Event::RedrawEventsCleared = &event {
                let now = Instant::now();
                let dt = (now - ctx.fps_timer).as_secs_f32();
                ctx.fps_timer = now;
                ctx.tracker.fps = ctx.tracker.fps.lerp(1.0 / dt, 0.1);
                if let Some(app) = &mut app {
                    if let Err(e) = ctx.draw(|drawer| Self::draw(drawer, app, &ctx)) {
                        Self::handle_error(e, app, &mut ctx)
                    }
                }
            }
            // Handle events
            for event in Event::from_glutin(event, &mut ctx.tracker, &mut ctx.camera) {
                let automatic_close = event == Event::CloseRequest && automatic_close;
                if automatic_close || ctx.should_close {
                    *cf = event_loop::ControlFlow::Exit;
                    if let Some(app) = app.take() {
                        Self::teardown(app, &mut ctx);
                    }
                    break;
                } else if let Some(app) = &mut app {
                    // Run app event method
                    if let Err(e) = Self::event(event, app, &mut ctx) {
                        Self::handle_error(e, app, &mut ctx);
                    }
                    // Run event scripts
                    #[cfg(feature = "script")]
                    if let Ok(scripts) = ctx.scripts() {
                        if let Err(e) = scripts.batch_call("event", move |lua, t, f| {
                            let mut ser = crate::LuaSerializer::new(lua);
                            let event = ser.serialize(&event)?;
                            f.call((t, event))?;
                            Ok(())
                        }) {
                            Self::handle_error(e, app, &mut ctx);
                        }
                    }
                }
            }
            // Update
            let now = Instant::now();
            let dt = (now - ctx.update_timer).as_secs_f32();
            if dt >= 1.0 / update_frequency {
                ctx.update_timer = now;
                if let Some(app) = &mut app {
                    // Run app update method
                    if let Err(e) = Self::update(dt, app, &mut ctx) {
                        Self::handle_error(e, app, &mut ctx);
                    }
                    // Run update scripts
                    #[cfg(feature = "script")]
                    if let Ok(scripts) = ctx.scripts() {
                        if let Err(e) = scripts.batch_call("update", move |_, t, f| {
                            f.call((t, dt))?;
                            Ok(())
                        }) {
                            Self::handle_error(e, app, &mut ctx);
                        }
                    }
                }
            }
        })
    }
}

/// Resource id types for an app
pub trait Resources: Copy + Eq + Hash {
    /// The id used to identify fonts
    type FontId: ResourceId;
    /// The id used to identify irregular cached meshes
    type MeshId: ResourceId;
    /// The id used to identify sounds
    type SoundId: ResourceId;
}

impl Resources for () {
    type FontId = ();
    type MeshId = ();
    type SoundId = ();
}

/// An id for app resources
pub trait ResourceId: Copy + Eq + Hash + Debug {}

impl<T> ResourceId for T where T: Copy + Eq + Hash + Debug {}

/**
A generic resources type

This type makes it easy to construct your own resources type

```
# use kule::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FontId {
    ComicSans,
    Papyrus
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MeshId(u32);

type MyRecs = GenericResources<FontId, MeshId, ()>;
```
*/
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericResources<FontId, MeshId, SoundId>(
    PhantomData<FontId>,
    PhantomData<MeshId>,
    PhantomData<SoundId>,
);

impl<F, M, S> Resources for GenericResources<F, M, S>
where
    F: ResourceId,
    M: ResourceId,
    S: ResourceId,
{
    type FontId = F;
    type MeshId = M;
    type SoundId = S;
}
