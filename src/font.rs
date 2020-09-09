use std::{collections::HashMap, mem::transmute};

use fontdue::*;

pub use fontdue::Metrics;

pub struct Fonts<G>(HashMap<G, GlyphCache>);

impl<G> Default for Fonts<G> {
    fn default() -> Self {
        Fonts(HashMap::default())
    }
}

impl<G> Fonts<G>
where
    G: Eq + std::hash::Hash,
{
    pub fn load(&mut self, id: G, data: &[u8]) -> crate::Result<()> {
        self.0.insert(
            id,
            Font::from_bytes(data, Default::default())
                .map_err(crate::Error::Static)?
                .into(),
        );
        Ok(())
    }
    pub fn get(&mut self, id: G) -> Option<&mut GlyphCache> {
        self.0.get_mut(&id)
    }
}

pub struct GlyphCache {
    font: Font,
    raster: HashMap<(char, u32), (Metrics, Vec<u8>)>,
}

impl From<Font> for GlyphCache {
    fn from(font: Font) -> Self {
        GlyphCache {
            font,
            raster: HashMap::new(),
        }
    }
}

impl GlyphCache {
    pub fn font(&self) -> &Font {
        &self.font
    }
    #[allow(clippy::transmute_float_to_int)]
    pub(crate) fn rasterize(&mut self, ch: char, size: f32) -> &(Metrics, Vec<u8>) {
        let size_u32: u32 = unsafe { transmute(size) };
        let font = &self.font;
        self.raster
            .entry((ch, size_u32))
            .or_insert_with(|| font.rasterize(ch, size))
    }
    pub fn metrics(&mut self, ch: char, size: f32) -> &Metrics {
        &self.rasterize(ch, size).0
    }
}
