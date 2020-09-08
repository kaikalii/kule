use std::{cell::RefCell, time::Instant};

use glium::{glutin::*, *};
use vector2math::*;

pub use window::WindowId;

use crate::{Camera, Drawer, Event, Fonts, StateTracker, Vec2};

pub trait App: Sized {
    fn builder() -> WindowBuilder<Self> {
        Window::builder()
    }
}

impl<T> App for T where T: Sized {}

pub struct WindowInner {
    display: Display,
    update_timer: Instant,
}

pub struct Window<T, G = ()> {
    pub app: T,
    pub program: Program,
    pub tracker: StateTracker,
    pub camera: Camera,
    glyphs: RefCell<Fonts<G>>,
    inner: WindowInner,
}

impl<T, G> Window<T, G> {
    pub fn builder() -> WindowBuilder<T, G> {
        WindowBuilder::default()
    }
    pub fn mouse_coords(&self) -> Vec2 {
        self.camera.pos_to_coords(self.tracker.mouse_pos())
    }
    fn _window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&window::Window) -> R,
    {
        f(self.inner.display.gl_window().window())
    }
    fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<Frame, Display, G>),
    {
        let mut frame = self.inner.display.draw();
        let mut glyphs = self.glyphs.borrow_mut();
        let mut drawer = Drawer::new(
            &mut frame,
            &self.inner.display,
            &self.program,
            &mut *glyphs,
            self.camera,
        );
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

impl<T, G> Window<T, G>
where
    G: Eq + std::hash::Hash,
{
    pub fn load_font(&self, id: G, bytes: &[u8]) -> crate::Result<()> {
        self.glyphs.borrow_mut().load(id, bytes)
    }
}

type Callback<F> = Option<Box<F>>;

#[allow(clippy::type_complexity)]
pub struct WindowBuilder<T, G = ()> {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
    pub setup: Callback<dyn FnOnce(&mut Window<T, G>)>,
    pub draw: Callback<dyn Fn(&mut Drawer<Frame, Display, G>, &Window<T, G>)>,
    pub event: Callback<dyn Fn(Event, &mut Window<T, G>)>,
    pub update: Callback<dyn Fn(f32, &mut Window<T, G>)>,
    pub update_frequency: f32,
    pub samples: u16,
}

impl<T, G> Default for WindowBuilder<T, G> {
    fn default() -> Self {
        WindowBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
            automatic_close: true,
            setup: None,
            draw: None,
            event: None,
            update: None,
            update_frequency: 120.0,
            samples: 0,
        }
    }
}

impl<T, G> WindowBuilder<T, G>
where
    T: 'static,
    G: 'static,
{
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
            .with_inner_size(dpi::LogicalSize::new(self.size[0], self.size[1]));
        let cb = ContextBuilder::new().with_multisampling(self.samples);
        let display = Display::new(wb, cb, &event_loop)?;
        let window_size = display.gl_window().window().inner_size();
        let program = crate::default_shaders(&display);
        let mut window = Window {
            app,
            inner: WindowInner {
                display,
                update_timer: Instant::now(),
            },
            program,
            glyphs: Default::default(),
            tracker: StateTracker::new(),
            camera: Camera {
                center: [0.0; 2],
                zoom: [1.0; 2],
                window_size: window_size.into(),
            },
        };
        if let Some(setup) = self.setup.take() {
            setup(&mut window)
        }
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            // Draw
            if let event::Event::RedrawEventsCleared = &event {
                if let Some(draw) = &self.draw {
                    window.draw(|drawer| draw(drawer, &window));
                }
            }
            // Handle events
            for event in Event::from_glutin(event, &mut window.tracker, &mut window.camera) {
                if let (Event::CloseRequest, true) = (event, self.automatic_close) {
                    *cf = event_loop::ControlFlow::Exit;
                    break;
                } else if let Some(handle_event) = &self.event {
                    handle_event(event, &mut window);
                }
            }
            // Update
            if let Some(update) = &self.update {
                let now = Instant::now();
                let dt = (now - window.inner.update_timer).as_secs_f32();
                if dt >= 1.0 / self.update_frequency {
                    window.inner.update_timer = now;
                    update(dt, &mut window);
                }
            }
        })
    }
    pub fn title<S>(self, title: S) -> Self
    where
        S: Into<String>,
    {
        WindowBuilder {
            title: title.into(),
            ..self
        }
    }
    pub fn size<V>(self, size: V) -> Self
    where
        V: Vector2<Scalar = f32>,
    {
        WindowBuilder {
            size: size.map(),
            ..self
        }
    }
    pub fn automatic_close(self, automatic_close: bool) -> Self {
        WindowBuilder {
            automatic_close,
            ..self
        }
    }
    pub fn samples(self, samples: u16) -> Self {
        WindowBuilder { samples, ..self }
    }
    pub fn setup<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Window<T, G>) + 'static,
    {
        WindowBuilder {
            setup: Some(Box::new(f)),
            ..self
        }
    }
    pub fn draw<F>(self, f: F) -> Self
    where
        F: Fn(&mut Drawer<Frame, Display, G>, &Window<T, G>) + 'static,
    {
        WindowBuilder {
            draw: Some(Box::new(f)),
            ..self
        }
    }
    pub fn event<F>(self, f: F) -> Self
    where
        F: Fn(Event, &mut Window<T, G>) + 'static,
    {
        WindowBuilder {
            event: Some(Box::new(f)),
            ..self
        }
    }
    pub fn update<F>(self, f: F) -> Self
    where
        F: Fn(f32, &mut Window<T, G>) + 'static,
    {
        WindowBuilder {
            update: Some(Box::new(f)),
            ..self
        }
    }
}
