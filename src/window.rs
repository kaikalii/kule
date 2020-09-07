use glium::{glutin::*, *};
use vector2math::*;

pub use window::WindowId;

pub struct Window {
    display: Display,
}

impl Window {
    pub fn builder() -> WindowBuilder {
        WindowBuilder::default()
    }
}

pub struct WindowBuilder {
    pub title: String,
    pub size: [f32; 2],
}

impl Default for WindowBuilder {
    fn default() -> Self {
        WindowBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
        }
    }
}

impl WindowBuilder {
    fn build(self) -> (event_loop::EventLoop<()>, Window) {
        #[cfg(not(test))]
        let event_loop = event_loop::EventLoop::new();
        #[cfg(test)]
        let event_loop = {
            use platform::windows::EventLoopExtWindows;
            event_loop::EventLoop::<()>::new_any_thread()
        };
        let wb = window::WindowBuilder::new()
            .with_title(self.title)
            .with_inner_size(dpi::LogicalSize::new(self.size[0], self.size[1]));
        let cb = ContextBuilder::new();
        let display = Display::new(wb, cb, &event_loop).unwrap();
        let window = Window { display };
        (event_loop, window)
    }
    pub fn run<F>(self, f: F)
    where
        F: Fn(Window) -> Window + std::panic::RefUnwindSafe + 'static,
    {
        let (event_loop, window) = self.build();
        let mut window = Some(window);
        event_loop.run(move |event, _, cf| {
            match event {
                event::Event::DeviceEvent { .. }
                | event::Event::MainEventsCleared
                | event::Event::NewEvents(_)
                | event::Event::RedrawEventsCleared => {}
                event::Event::WindowEvent {
                    event: event::WindowEvent::CloseRequested,
                    ..
                } => *cf = event_loop::ControlFlow::Exit,
                event => println!("{:?}", event),
            }
            window = Some(f(window.take().unwrap()));
        });
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
}
