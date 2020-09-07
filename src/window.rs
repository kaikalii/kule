use glium::{glutin::*, *};
use vector2math::*;

pub use window::WindowId;

use crate::{Event, StateTracker};

pub struct Window {
    display: Display,
    state: StateTracker,
}

impl Window {
    pub fn builder() -> WindowBuilder {
        WindowBuilder::default()
    }
    fn window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&window::Window) -> R,
    {
        f(self.display.gl_window().window())
    }
}

pub struct WindowBuilder {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
}

impl Default for WindowBuilder {
    fn default() -> Self {
        WindowBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
            automatic_close: true,
        }
    }
}

impl WindowBuilder {
    pub fn run<F>(self, f: F) -> crate::Result<()>
    where
        F: Fn(Window, Event) -> Window + std::panic::RefUnwindSafe + 'static,
    {
        // Build event loop and display
        #[cfg(not(test))]
        let event_loop = event_loop::EventLoop::new();
        #[cfg(test)]
        let event_loop = {
            use platform::windows::EventLoopExtWindows;
            event_loop::EventLoop::<()>::new_any_thread()
        };
        let wb = window::WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(dpi::LogicalSize::new(self.size[0], self.size[1]));
        let cb = ContextBuilder::new();
        let display = Display::new(wb, cb, &event_loop)?;
        let mut take_window = Some(Window {
            display,
            state: StateTracker::default(),
        });
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            let mut window = take_window.take().unwrap();
            for event in Event::from_glutin(event, &mut window.state) {
                if let (Event::CloseRequest, true) = (event, self.automatic_close) {
                    *cf = event_loop::ControlFlow::Exit;
                    break;
                } else {
                    window = f(window, event);
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
}
