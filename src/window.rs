use std::time::Instant;

use glium::{glutin::*, *};
use vector2math::*;

pub use window::WindowId;

use crate::{Camera, Drawer, Event, StateTracker, Vec2};

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

pub struct Window<T> {
    pub app: T,
    pub program: Program,
    pub tracker: StateTracker,
    pub camera: Camera,
    #[doc(hidden)]
    pub inner: WindowInner,
}

impl<T> Window<T> {
    pub fn builder() -> WindowBuilder<T> {
        WindowBuilder::default()
    }
    pub fn app<F>(self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        Window {
            app: f(self.app),
            ..self
        }
    }
    pub fn camera<F>(self, f: F) -> Self
    where
        F: FnOnce(Camera) -> Camera,
    {
        Window {
            camera: f(self.camera),
            ..self
        }
    }
    pub fn mouse_coords(&self) -> Vec2 {
        self.camera.pos_to_coords(self.tracker.mouse_pos)
    }
    fn _window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&window::Window) -> R,
    {
        f(self.inner.display.gl_window().window())
    }
    fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<Frame, Display>),
    {
        let mut frame = self.inner.display.draw();
        let mut drawer = Drawer::new(&mut frame, &self.inner.display, &self.program, self.camera);
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

type Callback<F> = Option<Box<F>>;

#[allow(clippy::type_complexity)]
pub struct WindowBuilder<T> {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
    pub setup: Callback<dyn FnOnce(Window<T>) -> Window<T>>,
    pub draw: Callback<dyn Fn(&mut Drawer<Frame, Display>, &Window<T>)>,
    pub event: Callback<dyn Fn(Event, Window<T>) -> Window<T>>,
    pub update: Callback<dyn Fn(f32, Window<T>) -> Window<T>>,
    pub update_frequency: f32,
}

impl<T> Default for WindowBuilder<T> {
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
        }
    }
}

impl<T> WindowBuilder<T>
where
    T: 'static,
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
        let cb = ContextBuilder::new();
        let display = Display::new(wb, cb, &event_loop)?;
        let window_size = display.gl_window().window().inner_size();
        let program = crate::default_shaders(&display);
        let window = Window {
            app,
            inner: WindowInner {
                display,
                update_timer: Instant::now(),
            },
            program,
            tracker: StateTracker::new(),
            camera: Camera {
                center: [0.0; 2],
                zoom: [1.0; 2],
                window_size: window_size.into(),
            },
        };
        let mut take_window = Some(if let Some(setup) = self.setup.take() {
            setup(window)
        } else {
            window
        });
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            let mut window = take_window.take().unwrap();
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
                    window = handle_event(event, window);
                }
            }
            // Update
            if let Some(update) = &self.update {
                let now = Instant::now();
                let dt = (now - window.inner.update_timer).as_secs_f32();
                if dt >= 1.0 / self.update_frequency {
                    window.inner.update_timer = now;
                    window = update(dt, window);
                }
            }
            take_window = Some(window);
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
    pub fn setup<F>(self, f: F) -> Self
    where
        F: FnOnce(Window<T>) -> Window<T> + 'static,
    {
        WindowBuilder {
            setup: Some(Box::new(f)),
            ..self
        }
    }
    pub fn draw<F>(self, f: F) -> Self
    where
        F: Fn(&mut Drawer<Frame, Display>, &Window<T>) + 'static,
    {
        WindowBuilder {
            draw: Some(Box::new(f)),
            ..self
        }
    }
    pub fn event<F>(self, f: F) -> Self
    where
        F: Fn(Event, Window<T>) -> Window<T> + 'static,
    {
        WindowBuilder {
            event: Some(Box::new(f)),
            ..self
        }
    }
    pub fn update<F>(self, f: F) -> Self
    where
        F: Fn(f32, Window<T>) -> Window<T> + 'static,
    {
        WindowBuilder {
            update: Some(Box::new(f)),
            ..self
        }
    }
}
