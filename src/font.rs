use std::{collections::HashMap, mem::transmute};

use fontdue::*;

pub(crate) type Fonts<G> = HashMap<G, GlyphCache>;

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
    #[allow(clippy::transmute_float_to_int)]
    pub(crate) fn rasterize(&mut self, ch: char, size: f32) -> &(Metrics, Vec<u8>) {
        let size_u32: u32 = unsafe { transmute(size) };
        let font = &self.font;
        self.raster
            .entry((ch, size_u32))
            .or_insert_with(|| font.rasterize(ch, size))
    }
}
