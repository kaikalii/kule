use std::{cell::Ref, time::Instant};

use glium::{glutin::*, *};
use vector2math::*;

pub use monitor::MonitorHandle;
pub use window::{Fullscreen, WindowId};

#[cfg(feature = "sound")]
use crate::{
    rodio::{Sample, Source},
    Kule, Mixer, SoundSource, Sounds,
};
use crate::{
    Camera, Drawer, Fonts, GlyphCache, KuleResult, MeshCache, Resources, StateTracker, Vec2,
    WindowCanvas,
};

/// A handle to the app's window
pub struct Window(pub(crate) Display);

impl Window {
    /// Get a reference to the inner window
    pub fn inner(&self) -> Ref<window::Window> {
        Ref::map(self.0.gl_window(), |gl_window| gl_window.window())
    }
    /// Get the position of the window
    pub fn position(&self) -> [i32; 2] {
        let pos = self.inner().outer_position().unwrap();
        [pos.x, pos.y]
    }
    /// Set the position of the window
    pub fn set_position(&self, pos: [i32; 2]) {
        self.inner()
            .set_outer_position(dpi::PhysicalPosition::<i32>::from(pos));
    }
    /// Get a handle to the window's current monitor
    pub fn current_monitor(&self) -> MonitorHandle {
        self.inner().current_monitor()
    }
    /// Set the window's fullscreen state
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
        self.inner().set_fullscreen(fullscreen)
    }
    /// Get the size of the window in pixels
    pub fn size(&self) -> [u32; 2] {
        let size = self.inner().inner_size();
        [size.width, size.height]
    }
    /// Set the size of the window in pixels
    pub fn set_size(&self, size: [u32; 2]) {
        self.inner()
            .set_inner_size(dpi::PhysicalSize::<u32>::from(size));
    }
    /// Get whether the cursor should be visible
    pub fn cursor_visible(&self) -> bool {
        todo!()
    }
    /// Set whether the cursor should be visible
    pub fn set_cursor_visible(&self, visible: bool) {
        self.inner().set_cursor_visible(visible);
    }
    /// Set the window icon using bitmap data
    pub fn set_icon(&self, rgba: Vec<u8>, width: u32, height: u32) -> KuleResult<()> {
        self.inner()
            .set_window_icon(Some(window::Icon::from_rgba(rgba, width, height)?));
        Ok(())
    }
}

/// Holds the state of the engine
pub struct Context<R = ()>
where
    R: Resources,
{
    /// The main shader to use for drawing
    pub program: Program,
    /// Tracks the state of various inputs
    pub tracker: StateTracker,
    /// The scene camera
    pub camera: Camera,
    /// A handle to the window
    pub window: Window,
    /// The font cache
    pub fonts: Fonts<R::FontId>,
    /// The mesh cache
    pub meshes: MeshCache<R>,
    #[cfg(feature = "sound")]
    /// The audio mixer
    pub mixer: Mixer,
    #[cfg(feature = "sound")]
    /// The sound cache
    pub sounds: Sounds<R::SoundId>,
    /// Whether the window should close
    pub should_close: bool,
    pub(crate) update_timer: Instant,
    pub(crate) fps_timer: Instant,
}

impl<R> Context<R>
where
    R: Resources,
{
    /// Get the world coordinates of the mouse cursor
    pub fn mouse_coords(&self) -> Vec2 {
        self.camera.pos_to_coords(self.tracker.mouse_pos())
    }
    pub(crate) fn draw<F>(&self, mut f: F)
    where
        F: FnMut(&mut Drawer<WindowCanvas, R>),
    {
        let mut frame = self.window.0.draw();
        let mut drawer = Drawer::new(
            &mut frame,
            &self.window.0,
            &self.program,
            &self.fonts,
            &self.meshes,
            self.camera,
        );
        f(&mut drawer);
        frame.finish().unwrap();
    }
}

impl<R> Context<R>
where
    R: Resources,
{
    /// Load a font
    pub fn load_font(&mut self, font_id: R::FontId, bytes: &[u8]) -> KuleResult<()> {
        self.fonts.load(font_id, bytes)
    }
    /**
    Get the glyph cache for a font

    # Panics

    Panics if no font is loaded for the given font id
    */
    pub fn glyphs(&self, font_id: R::FontId) -> &GlyphCache {
        self.get_glyphs(font_id)
            .expect("No font loaded for font id")
    }
    /// Get the glyph cache for a font
    pub fn get_glyphs(&self, font_id: R::FontId) -> Option<&GlyphCache> {
        self.fonts.get(font_id)
    }
    #[cfg(feature = "sound")]
    /// Play an id'd sound
    pub fn play_sound<A>(&mut self, sound_id: R::SoundId, app: &A) -> KuleResult<()>
    where
        A: Kule<Resources = R>,
    {
        self.play_modified_sound(sound_id, app, |s| s)
    }
    #[cfg(feature = "sound")]
    /// Play an id'd sound with a modified `Source`
    pub fn play_modified_sound<A, F, S>(
        &mut self,
        sound_id: R::SoundId,
        app: &A,
        f: F,
    ) -> KuleResult<()>
    where
        A: Kule<Resources = R>,
        F: Fn(SoundSource) -> S,
        S: Source + Send + 'static,
        S::Item: Sample,
    {
        if !self.sounds.contains(sound_id) {
            if let Some(buffer) = A::load_sound(sound_id, app)? {
                self.sounds.insert(sound_id, buffer);
            }
        }
        if let Some(buffer) = self.sounds.get(sound_id) {
            self.mixer.play(f(SoundSource::from(buffer.clone())));
        }
        Ok(())
    }
}

impl<R> Context<R>
where
    R: Resources<FontId = ()>,
{
    /// Load the only font if `Resources::FontId` is `()`
    pub fn load_only_font(&mut self, bytes: &[u8]) -> KuleResult<()> {
        self.load_font((), bytes)
    }
    /// Load the only glyph cache if `Resources::FontId` is `()`
    pub fn only_glyphs(&self) -> &GlyphCache {
        self.glyphs(())
    }
}

/// The primary structure for defining your app's behavior
#[allow(clippy::type_complexity)]
pub struct ContextBuilder {
    /// The window title
    pub title: String,
    /// The window size
    pub size: [f32; 2],
    /// Whether the window should automatically close when clicking the "X"
    pub automatic_close: bool,
    /// How often to call the app's `update` function in Hz
    pub update_frequency: f32,
    /// Samples to use for antialiasing
    pub samples: u16,
    /// The window's icon
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
    /// Create a new `ContextBuilder`
    pub fn new() -> Self {
        ContextBuilder::default()
    }
    /// Set the title
    pub fn title<S>(self, title: S) -> Self
    where
        S: Into<String>,
    {
        ContextBuilder {
            title: title.into(),
            ..self
        }
    }
    /// Set the window size
    pub fn size<V>(self, size: V) -> Self
    where
        V: Vector2<Scalar = f32>,
    {
        ContextBuilder {
            size: size.map(),
            ..self
        }
    }
    /// Set whether the window should automatically close when clicking the "X"
    pub fn automatic_close(self, automatic_close: bool) -> Self {
        ContextBuilder {
            automatic_close,
            ..self
        }
    }
    /// Set the samples used for antialiasing
    pub fn samples(self, samples: u16) -> Self {
        ContextBuilder { samples, ..self }
    }
    /// Set the window icon using bitmap data
    pub fn icon(self, rgba: Vec<u8>, width: u32, height: u32) -> KuleResult<Self> {
        Ok(ContextBuilder {
            icon: Some(window::Icon::from_rgba(rgba, width, height)?),
            ..self
        })
    }
}
