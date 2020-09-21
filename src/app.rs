use std::{fmt::Debug, hash::Hash, marker::PhantomData, time::Instant};

use glium::{glutin::*, *};

use crate::{
    Camera, Canvas, Context, ContextBuilder, Drawer, Event, FloatingScalar, KuleResult,
    StateTracker, Window,
};

#[allow(unused_variables)]
pub trait Kule: Sized + 'static {
    type Resources: Resources;
    fn build() -> ContextBuilder {
        ContextBuilder::default()
    }
    fn setup(ctx: &mut Context<Self::Resources>) -> Self;
    fn update(dt: f32, app: &mut Self, ctx: &mut Context<Self::Resources>) {}
    fn draw<C>(draw: &mut Drawer<C, Self::Resources>, app: &Self, ctx: &Context<Self::Resources>)
    where
        C: Canvas,
    {
    }
    fn event(event: Event, app: &mut Self, ctx: &mut Context<Self::Resources>) {}
    fn teardown(app: Self, ctx: &mut Context<Self::Resources>) {}
    fn run() -> KuleResult<()> {
        let mut builder = Self::build();
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
            .with_title(&builder.title)
            .with_window_icon(builder.icon.take())
            .with_inner_size(dpi::LogicalSize::new(builder.size[0], builder.size[1]));
        let cb = glutin::ContextBuilder::new().with_multisampling(builder.samples);
        let display = Display::new(wb, cb, &event_loop)?;
        println!("{:?}", display.get_supported_glsl_version());
        let window_size = display.gl_window().window().inner_size();
        let program = crate::default_shaders(&display);
        let mut ctx = Context {
            program,
            fonts: Default::default(),
            tracker: StateTracker::new(),
            camera: Camera {
                center: [0.0; 2],
                zoom: 1.0,
                window_size: window_size.into(),
            },
            window: Window(display),
            should_close: false,
            update_timer: Instant::now(),
            fps_timer: Instant::now(),
        };
        let app = Self::setup(&mut ctx);
        let mut app = Some(app);
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            // Draw
            if let event::Event::RedrawEventsCleared = &event {
                let now = Instant::now();
                let dt = (now - ctx.fps_timer).as_secs_f32();
                ctx.fps_timer = now;
                ctx.tracker.fps = ctx.tracker.fps.lerp(1.0 / dt, 0.1);
                if let Some(app) = &app {
                    ctx.draw(|drawer| Self::draw(drawer, app, &ctx));
                }
            }
            // Handle events
            for event in Event::from_glutin(event, &mut ctx.tracker, &mut ctx.camera) {
                let automatic_close = event == Event::CloseRequest && builder.automatic_close;
                if automatic_close || ctx.should_close {
                    *cf = event_loop::ControlFlow::Exit;
                    if let Some(app) = app.take() {
                        Self::teardown(app, &mut ctx);
                    }
                    break;
                } else if let Some(app) = &mut app {
                    Self::event(event, app, &mut ctx);
                }
            }
            // Update
            let now = Instant::now();
            let dt = (now - ctx.update_timer).as_secs_f32();
            if dt >= 1.0 / builder.update_frequency {
                ctx.update_timer = now;
                if let Some(app) = &mut app {
                    Self::update(dt, app, &mut ctx);
                }
            }
        })
    }
}

pub trait Resources: Copy + Eq + Hash {
    type FontId: ResourceId;
    type MeshId: ResourceId;
}

impl Resources for () {
    type FontId = ();
    type MeshId = ();
}

pub trait ResourceId: Copy + Eq + Hash + Debug {}

impl<T> ResourceId for T where T: Copy + Eq + Hash + Debug {}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericResources<F, M>(PhantomData<F>, PhantomData<M>);

impl<F, M> Resources for GenericResources<F, M>
where
    F: ResourceId,
    M: ResourceId,
{
    type FontId = F;
    type MeshId = M;
}

pub type VaryMeshes<M> = GenericResources<(), M>;
