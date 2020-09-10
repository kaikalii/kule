use std::{
    cell::{Ref, RefCell, RefMut},
    time::Instant,
};

use glium::{glutin::*, *};
use vector2math::*;

pub use monitor::MonitorHandle;
pub use window::{Fullscreen, WindowId};

use crate::{Camera, Drawer, Event, Fonts, GlyphCache, StateTracker, Vec2};

pub struct Window(Display);

impl Window {
    pub fn inner(&self) -> Ref<window::Window> {
        Ref::map(self.0.gl_window(), |gl_window| gl_window.window())
    }
    pub fn position(&self) -> [i32; 2] {
        let pos = self.inner().outer_position().unwrap();
        [pos.x, pos.y]
    }
    pub fn set_position(&self, pos: [i32; 2]) {
        self.inner()
            .set_outer_position(dpi::PhysicalPosition::<i32>::from(pos));
    }
    pub fn current_monitor(&self) -> MonitorHandle {
        self.inner().current_monitor()
    }
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
        self.inner().set_fullscreen(fullscreen)
    }
    pub fn size(&self) -> [u32; 2] {
        let size = self.inner().inner_size();
        [size.width, size.height]
    }
    pub fn set_size(&self, size: [u32; 2]) {
        self.inner()
            .set_inner_size(dpi::PhysicalSize::<u32>::from(size));
    }
    pub fn set_cursor_visible(&self, visible: bool) {
        self.inner().set_cursor_visible(visible);
    }
    pub fn set_icon(&self, rgba: Vec<u8>, width: u32, height: u32) -> crate::Result<()> {
        self.inner()
            .set_window_icon(Some(window::Icon::from_rgba(rgba, width, height)?));
        Ok(())
    }
}

pub struct Context<T, G = ()> {
    pub app: T,
    pub program: Program,
    pub tracker: StateTracker,
    pub camera: Camera,
    pub window: Window,
    fonts: RefCell<Fonts<G>>,
    update_timer: Instant,
}

impl<T, G> Context<T, G> {
    pub fn mouse_coords(&self) -> Vec2 {
        self.camera.pos_to_coords(self.tracker.mouse_pos())
    }
    fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<Frame, Display, G>),
    {
        let mut frame = self.window.0.draw();
        let mut fonts = self.fonts.borrow_mut();
        let mut drawer = Drawer::new(
            &mut frame,
            &self.window.0,
            &self.program,
            &mut *fonts,
            self.camera,
        );
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

impl<T, G> Context<T, G>
where
    G: Copy + Eq + std::hash::Hash + std::fmt::Debug,
{
    pub fn load_font(&self, font_id: G, bytes: &[u8]) -> crate::Result<()> {
        self.fonts.borrow_mut().load(font_id, bytes)
    }
    pub fn glyphs(&self, font_id: G) -> RefMut<GlyphCache> {
        RefMut::map(self.fonts.borrow_mut(), |fonts| fonts.get(font_id).unwrap())
    }
    pub fn get_glyphs(&self, font_id: G) -> Option<RefMut<GlyphCache>> {
        if self.fonts.borrow_mut().get(font_id).is_some() {
            Some(RefMut::map(self.fonts.borrow_mut(), |fonts| {
                fonts.get(font_id).unwrap()
            }))
        } else {
            None
        }
    }
}

impl<T> Context<T> {
    pub fn load_only_font(&self, bytes: &[u8]) -> crate::Result<()> {
        self.load_font((), bytes)
    }
    pub fn only_glyphs(&self) -> RefMut<GlyphCache> {
        self.glyphs(())
    }
}

type Callback<F> = Option<Box<F>>;

/// The primary structure for defining your app's behavior
#[allow(clippy::type_complexity)]
pub struct AppBuilder<T, G = ()> {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
    pub setup: Callback<dyn FnOnce(&mut Context<T, G>)>,
    pub draw: Callback<dyn Fn(&mut Drawer<Frame, Display, G>, &Context<T, G>)>,
    pub event: Callback<dyn Fn(Event, &mut Context<T, G>)>,
    pub update: Callback<dyn Fn(f32, &mut Context<T, G>)>,
    pub update_frequency: f32,
    pub samples: u16,
    pub icon: Option<window::Icon>,
}

impl<T, G> Default for AppBuilder<T, G> {
    fn default() -> Self {
        AppBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
            automatic_close: true,
            setup: None,
            draw: None,
            event: None,
            update: None,
            update_frequency: 120.0,
            samples: 0,
            icon: None,
        }
    }
}

impl<T, G> AppBuilder<T, G>
where
    T: 'static,
    G: 'static,
{
    pub fn new() -> Self {
        AppBuilder::default()
    }
    pub fn run(mut self, app: T) -> crate::Result<()> {
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
            .with_title(&self.title)
            .with_window_icon(self.icon.take())
            .with_inner_size(dpi::LogicalSize::new(self.size[0], self.size[1]));
        let cb = ContextBuilder::new().with_multisampling(self.samples);
        let display = Display::new(wb, cb, &event_loop)?;
        let window_size = display.gl_window().window().inner_size();
        let program = crate::default_shaders(&display);
        let mut ctx = Context {
            app,
            program,
            fonts: Default::default(),
            tracker: StateTracker::new(),
            camera: Camera {
                center: [0.0; 2],
                zoom: [1.0; 2],
                window_size: window_size.into(),
            },
            window: Window(display),
            update_timer: Instant::now(),
        };
        if let Some(setup) = self.setup.take() {
            setup(&mut ctx)
        }
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            // Draw
            if let event::Event::RedrawEventsCleared = &event {
                if let Some(draw) = &self.draw {
                    ctx.draw(|drawer| draw(drawer, &ctx));
                }
            }
            // Handle events
            for event in Event::from_glutin(event, &mut ctx.tracker, &mut ctx.camera) {
                if let (Event::CloseRequest, true) = (event, self.automatic_close) {
                    *cf = event_loop::ControlFlow::Exit;
                    break;
                } else if let Some(handle_event) = &self.event {
                    handle_event(event, &mut ctx);
                }
            }
            // Update
            if let Some(update) = &self.update {
                let now = Instant::now();
                let dt = (now - ctx.update_timer).as_secs_f32();
                if dt >= 1.0 / self.update_frequency {
                    ctx.update_timer = now;
                    update(dt, &mut ctx);
                }
            }
        })
    }
    pub fn title<S>(self, title: S) -> Self
    where
        S: Into<String>,
    {
        AppBuilder {
            title: title.into(),
            ..self
        }
    }
    pub fn size<V>(self, size: V) -> Self
    where
        V: Vector2<Scalar = f32>,
    {
        AppBuilder {
            size: size.map(),
            ..self
        }
    }
    pub fn automatic_close(self, automatic_close: bool) -> Self {
        AppBuilder {
            automatic_close,
            ..self
        }
    }
    pub fn samples(self, samples: u16) -> Self {
        AppBuilder { samples, ..self }
    }
    pub fn icon(self, rgba: Vec<u8>, width: u32, height: u32) -> crate::Result<Self> {
        Ok(AppBuilder {
            icon: Some(window::Icon::from_rgba(rgba, width, height)?),
            ..self
        })
    }
    pub fn setup<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Context<T, G>) + 'static,
    {
        AppBuilder {
            setup: Some(Box::new(f)),
            ..self
        }
    }
    pub fn draw<F>(self, f: F) -> Self
    where
        F: Fn(&mut Drawer<Frame, Display, G>, &Context<T, G>) + 'static,
    {
        AppBuilder {
            draw: Some(Box::new(f)),
            ..self
        }
    }
    pub fn event<F>(self, f: F) -> Self
    where
        F: Fn(Event, &mut Context<T, G>) + 'static,
    {
        AppBuilder {
            event: Some(Box::new(f)),
            ..self
        }
    }
    pub fn update<F>(self, f: F) -> Self
    where
        F: Fn(f32, &mut Context<T, G>) + 'static,
    {
        AppBuilder {
            update: Some(Box::new(f)),
            ..self
        }
    }
}
