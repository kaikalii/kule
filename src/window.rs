use glium::{glutin::*, *};
use vector2math::*;

pub use window::WindowId;

use crate::{Drawer, Event, StateTracker};

pub struct Window {
    display: Display,
    program: Program,
    state: StateTracker,
}

impl Window {
    pub fn builder() -> WindowBuilder {
        WindowBuilder::default()
    }
    fn _window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&window::Window) -> R,
    {
        f(self.display.gl_window().window())
    }
    fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<Frame, Display>),
    {
        let mut frame = self.display.draw();
        let mut drawer = Drawer::new(&mut frame, &self.display, &self.program);
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

type Callback<F> = Option<Box<F>>;

pub struct WindowBuilder {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
    pub startup: Callback<dyn FnOnce(Window) -> Window>,
    pub draw: Callback<dyn Fn(&mut Drawer<Frame, Display>)>,
    pub update: Callback<dyn Fn(Window, Event) -> Window>,
}

impl Default for WindowBuilder {
    fn default() -> Self {
        WindowBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
            automatic_close: true,
            startup: None,
            draw: None,
            update: None,
        }
    }
}

impl WindowBuilder {
    pub fn run(mut self) -> crate::Result<()> {
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
        let program = crate::default_shaders(&display);
        let window = Window {
            display,
            program,
            state: StateTracker::default(),
        };
        let mut take_window = Some(if let Some(startup) = self.startup.take() {
            startup(window)
        } else {
            window
        });
        // Run the event loop
        event_loop.run(move |event, _, cf| {
            let mut window = take_window.take().unwrap();
            // Draw
            if let event::Event::RedrawRequested(_) = &event {
                if let Some(draw) = &self.draw {
                    window.draw(draw);
                }
            }
            // Update
            if let Some(update) = &self.update {
                for event in Event::from_glutin(event, &mut window.state) {
                    if let (Event::CloseRequest, true) = (event, self.automatic_close) {
                        *cf = event_loop::ControlFlow::Exit;
                        break;
                    } else {
                        window = update(window, event);
                    }
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
    pub fn startup<F>(self, f: F) -> Self
    where
        F: FnOnce(Window) -> Window + 'static,
    {
        WindowBuilder {
            startup: Some(Box::new(f)),
            ..self
        }
    }
    pub fn draw<F>(self, f: F) -> Self
    where
        F: Fn(&mut Drawer<Frame, Display>) + 'static,
    {
        WindowBuilder {
            draw: Some(Box::new(f)),
            ..self
        }
    }
    pub fn update<F>(self, f: F) -> Self
    where
        F: Fn(Window, Event) -> Window + 'static,
    {
        WindowBuilder {
            update: Some(Box::new(f)),
            ..self
        }
    }
}
