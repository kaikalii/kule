use std::{
    collections::{HashMap, HashSet},
    iter::once,
    rc::Rc,
};

use fontdue::*;
use lyon_tessellation::{
    geom::math::{point, Point},
    geometry_builder::simple_builder,
    path::Path,
    FillOptions, FillTessellator, VertexBuffers,
};

use crate::Vec2;

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

#[derive(Debug, Clone)]
pub struct GlyphGeometry {
    pub vertices: Vec<Vec2>,
    pub indices: Rc<Vec<u16>>,
}

pub struct GlyphCache {
    font: Font,
    geometry: HashMap<(char, u32), (Metrics, GlyphGeometry)>,
}

impl From<Font> for GlyphCache {
    fn from(font: Font) -> Self {
        GlyphCache {
            font,
            geometry: HashMap::new(),
        }
    }
}

impl GlyphCache {
    pub fn font(&self) -> &Font {
        &self.font
    }
    pub fn metrics(&mut self, ch: char, resolution: u32) -> &Metrics {
        &self.glyph(ch, resolution).0
    }
    #[allow(clippy::map_entry)]
    pub fn glyph(&mut self, ch: char, resolution: u32) -> &(Metrics, GlyphGeometry) {
        if !self.geometry.contains_key(&(ch, resolution)) {
            let glyph_data = self.vectorize(ch, resolution);
            self.geometry.insert((ch, resolution), glyph_data);
        }
        self.geometry.get(&(ch, resolution)).unwrap()
    }
    fn vectorize(&mut self, ch: char, resolution: u32) -> (Metrics, GlyphGeometry) {
        let (metrics, bytes) = self.font.rasterize(ch, resolution as f32);
        let get = |[x, y]: [usize; 2]| bytes[y * metrics.width + x] > 0;
        let mut edges = HashSet::new();
        // Collect relevant edge pixels
        for (i, b) in bytes.iter().enumerate() {
            let p = [i % metrics.width, i / metrics.width];
            if b == &0 || edges.contains(&p) {
                continue;
            }
            let empty_count = neighbors(p, metrics.width, metrics.height)
                .filter(|n| n.map_or(true, |n| !get(n)))
                .count();
            let empty_adj_count = adj_neighbors(p, metrics.width, metrics.height)
                .filter(|n| n.map_or(true, |n| !get(n)))
                .count();
            if 2 <= empty_count && empty_count <= 4 || 1 == empty_adj_count {
                edges.insert(p);
            }
        }

        let mut polys: Vec<Vec<[usize; 2]>> = Vec::new();
        // Group edges into polygons
        while let Some(first) = edges.iter().next().copied() {
            edges.remove(&first);
            polys.push(vec![first]);
            let poly = polys.last_mut().unwrap();
            loop {
                let p = poly.last().copied().unwrap();
                let neighbor_edges: Vec<[usize; 2]> = neighbors(p, metrics.width, metrics.height)
                    .filter_map(|n| n)
                    .filter(|e| edges.contains(e))
                    .collect();
                if neighbor_edges.is_empty() {
                    break;
                } else {
                    for ne in &neighbor_edges {
                        edges.remove(ne);
                    }
                    poly.extend(neighbor_edges.into_iter().max_by_key(|&[x, y]| {
                        p[0].max(x) - p[0].min(x) + p[1].max(y) - p[1].min(y)
                    }));
                }
            }
        }

        // Triangulate
        let mut path = Path::builder();
        for poly in polys {
            let mut poly_iter = poly.into_iter().map(|[x, y]| [x as f32, y as f32]);
            let [x, y] = poly_iter.next().unwrap();
            path.move_to(point(x, y));
            for [x, y] in poly_iter {
                path.line_to(point(x, y));
            }
            path.line_to(point(x, y));
            path.close();
        }
        let path = path.build();
        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tessellator = FillTessellator::new();
        tessellator
            .tessellate_path(&path, &FillOptions::default(), &mut vertex_builder)
            .unwrap();
        let indices = Rc::new(buffers.indices);
        let vertices: Vec<Vec2> = buffers.vertices.into_iter().map(|v| [v.x, v.y]).collect();
        (metrics, GlyphGeometry { indices, vertices })
    }
}

#[allow(clippy::many_single_char_names)]
fn adj_neighbors_array(p: [usize; 2], width: usize, height: usize) -> [Option<[usize; 2]>; 4] {
    let [x, y] = p;
    let l = if x > 0 { Some([x - 1, y]) } else { None };
    let r = if x < width - 1 {
        Some([x + 1, y])
    } else {
        None
    };
    let t = if y > 0 { Some([x, y - 1]) } else { None };
    let b = if y < height - 1 {
        Some([x, y + 1])
    } else {
        None
    };
    [l, r, t, b]
}

#[allow(clippy::many_single_char_names)]
fn adj_neighbors(
    p: [usize; 2],
    width: usize,
    height: usize,
) -> impl Iterator<Item = Option<[usize; 2]>> {
    let [l, r, t, b] = adj_neighbors_array(p, width, height);
    once(l).chain(once(r)).chain(once(t)).chain(once(b))
}

#[allow(clippy::many_single_char_names)]
fn neighbors(
    p: [usize; 2],
    width: usize,
    height: usize,
) -> impl Iterator<Item = Option<[usize; 2]>> {
    let [l, r, t, b] = adj_neighbors_array(p, width, height);
    let x1y2 = |([x, _], [_, y]): ([usize; 2], [usize; 2])| [x, y];
    let tl = l.zip(t).map(x1y2);
    let tr = r.zip(t).map(x1y2);
    let bl = l.zip(b).map(x1y2);
    let br = r.zip(b).map(x1y2);
    once(l)
        .chain(once(r))
        .chain(once(t))
        .chain(once(b))
        .chain(once(tl))
        .chain(once(tr))
        .chain(once(bl))
        .chain(once(br))
}
