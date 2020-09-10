use std::{cell::Ref, time::Instant};

use glium::{glutin::*, *};
use vector2math::*;

pub use monitor::MonitorHandle;
pub use window::{Fullscreen, WindowId};

use crate::{App, Camera, Drawer, Fonts, GlyphCache, KuleResult, StateTracker, Vec2};

pub struct Window(pub(crate) Display);

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
    pub fn set_icon(&self, rgba: Vec<u8>, width: u32, height: u32) -> KuleResult<()> {
        self.inner()
            .set_window_icon(Some(window::Icon::from_rgba(rgba, width, height)?));
        Ok(())
    }
}

pub struct Context<T>
where
    T: App,
{
    pub program: Program,
    pub tracker: StateTracker,
    pub camera: Camera,
    pub window: Window,
    pub fonts: Fonts<T::FontId>,
    pub should_close: bool,
    pub(crate) update_timer: Instant,
    pub(crate) fps_timer: Instant,
}

impl<T> Context<T>
where
    T: App,
{
    pub fn mouse_coords(&self) -> Vec2 {
        self.camera.pos_to_coords(self.tracker.mouse_pos())
    }
    pub(crate) fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<Frame, Display, T::FontId>),
    {
        let mut frame = self.window.0.draw();
        let mut drawer = Drawer::new(
            &mut frame,
            &self.window.0,
            &self.program,
            &self.fonts,
            self.camera,
        );
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

impl<T> Context<T>
where
    T: App,
{
    pub fn load_font(&mut self, font_id: T::FontId, bytes: &[u8]) -> KuleResult<()> {
        self.fonts.load(font_id, bytes)
    }
    pub fn glyphs(&self, font_id: T::FontId) -> &GlyphCache {
        self.get_glyphs(font_id)
            .expect("No font loaded for font id")
    }
    pub fn get_glyphs(&self, font_id: T::FontId) -> Option<&GlyphCache> {
        self.fonts.get(font_id)
    }
}

impl<T> Context<T>
where
    T: App<FontId = ()>,
{
    pub fn load_only_font(&mut self, bytes: &[u8]) -> KuleResult<()> {
        self.load_font((), bytes)
    }
    pub fn only_glyphs(&self) -> &GlyphCache {
        self.glyphs(())
    }
}

/// The primary structure for defining your app's behavior
#[allow(clippy::type_complexity)]
pub struct ContextBuilder {
    pub title: String,
    pub size: [f32; 2],
    pub automatic_close: bool,
    pub update_frequency: f32,
    pub samples: u16,
    pub icon: Option<window::Icon>,
}

impl Default for ContextBuilder {
    fn default() -> Self {
        ContextBuilder {
            title: env!("CARGO_CRATE_NAME").into(),
            size: [800.0; 2],
            automatic_close: true,
            update_frequency: 120.0,
            samples: 0,
            icon: None,
        }
    }
}

impl ContextBuilder {
    pub fn new() -> Self {
        ContextBuilder::default()
    }
    pub fn title<S>(self, title: S) -> Self
    where
        S: Into<String>,
    {
        ContextBuilder {
            title: title.into(),
            ..self
        }
    }
    pub fn size<V>(self, size: V) -> Self
    where
        V: Vector2<Scalar = f32>,
    {
        ContextBuilder {
            size: size.map(),
            ..self
        }
    }
    pub fn automatic_close(self, automatic_close: bool) -> Self {
        ContextBuilder {
            automatic_close,
            ..self
        }
    }
    pub fn samples(self, samples: u16) -> Self {
        ContextBuilder { samples, ..self }
    }
    pub fn icon(self, rgba: Vec<u8>, width: u32, height: u32) -> KuleResult<Self> {
        Ok(ContextBuilder {
            icon: Some(window::Icon::from_rgba(rgba, width, height)?),
            ..self
        })
    }
}
