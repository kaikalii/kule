use std::{collections::HashMap, iter::once, rc::Rc};

use glium::{backend::*, uniforms::*, *};
use vector2math::*;

use crate::{Col, Color, Fonts, GlyphSize, GlyphSpec, Rect, Resources, Trans, Vec2};

pub use index::PrimitiveType;

/// Satisfy a rust-analyzer bug
fn trans() -> Trans {
    Transform::new()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    pub pos: Vec2,
    pub color: Col,
}

implement_vertex!(Vertex, pos, color);

fn uniforms() -> UniformsStorage<'static, [[f32; 4]; 4], EmptyUniforms> {
    uniform! {
        matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub center: Vec2,
    pub zoom: f32,
    pub(crate) window_size: Vec2,
}

impl Camera {
    pub fn window_size(self) -> Vec2 {
        self.window_size
    }
    pub fn with_center(self, center: Vec2) -> Self {
        Camera { center, ..self }
    }
    pub fn with_zoom(self, zoom: f32) -> Self {
        Camera { zoom, ..self }
    }
    pub fn zoom(self, zoom: f32) -> Self {
        Camera {
            zoom: self.zoom * zoom,
            ..self
        }
    }
    pub fn bound_zoom(self, min: f32, max: f32) -> Self {
        Camera {
            zoom: self.zoom.max(min).min(max),
            ..self
        }
    }
    pub fn translate(self, offset: Vec2) -> Self {
        Camera {
            center: self.center.add(offset),
            ..self
        }
    }
    pub fn zoom_on(self, zoom: f32, on: Vec2) -> Self {
        let old_pos = self.pos_to_coords(on);
        let new_cam = self.zoom(zoom);
        let new_pos = new_cam.pos_to_coords(on);
        new_cam.translate(self.center.add(new_pos.sub(old_pos).neg()))
    }
    pub fn pos_to_coords(self, pos: Vec2) -> Vec2 {
        pos.sub(self.window_size.div(2.0))
            .div(self.zoom)
            // .mul(2.0)
            .add(self.center)
    }
    pub fn coords_to_pos(self, coords: Vec2) -> Vec2 {
        coords
            .sub(self.center)
            .div(2.0)
            .mul(self.zoom)
            .add(self.window_size.div(2.0))
    }
    pub fn view_rect(self) -> Rect {
        Rect::centered(self.center, self.window_size.div(self.zoom))
    }
    fn transform(&self) -> Trans {
        trans()
            .translate(self.center.neg())
            .scale([self.zoom; 2].mul2([1.0, -1.0]))
            .scale::<Vec2>(self.window_size.map_with(|d| 1.0 / d))
            .zoom(2.0)
    }
}

pub trait Canvas {
    type Facade: Facade;
    type Surface: Surface;
}

pub struct WindowCanvas;

impl Canvas for WindowCanvas {
    type Facade = Display;
    type Surface = Frame;
}

pub struct Drawer<'ctx, T = WindowCanvas, R = ()>
where
    T: Canvas,
    R: Resources,
{
    surface: &'ctx mut T::Surface,
    facade: &'ctx T::Facade,
    program: &'ctx Program,
    pub fonts: &'ctx Fonts<R::FontId>,
    pub camera: Camera,
    indices: IndicesCache,
}

impl<'ctx, T, R> Drawer<'ctx, T, R>
where
    T: Canvas,
    R: Resources,
{
    pub(crate) fn new(
        surface: &'ctx mut T::Surface,
        facade: &'ctx T::Facade,
        program: &'ctx Program,
        fonts: &'ctx Fonts<R::FontId>,
        camera: Camera,
    ) -> Self {
        Drawer {
            surface,
            facade,
            program,
            fonts,
            camera,
            indices: Default::default(),
        }
    }
    pub fn with_camera<C, F, S>(&mut self, camera: C, f: F) -> S
    where
        C: FnOnce(Camera) -> Camera,
        F: FnOnce(&mut Self) -> S,
    {
        let base_camera = self.camera;
        self.camera = camera(base_camera);
        let res = f(self);
        self.camera = base_camera;
        res
    }

    pub fn with_absolute_camera<F, S>(&mut self, f: F) -> S
    where
        F: FnOnce(&mut Self) -> S,
    {
        let base_camera = self.camera;
        self.with_camera(
            |_| Camera {
                center: base_camera.window_size.div(2.0),
                zoom: 1.0,
                window_size: base_camera.window_size,
            },
            f,
        )
    }
    pub fn clear<C>(&mut self, color: C)
    where
        C: Color,
    {
        self.surface
            .clear_color(color.r(), color.g(), color.b(), color.alpha())
    }
    pub fn rectangle<C, E>(&mut self, color: C, rect: E) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        E: Rectangle<Scalar = f32>,
    {
        Transformable::new(self, color.map(), once(DrawType::Rectangle(rect.map())))
    }
    pub fn circle<C, E>(
        &mut self,
        color: C,
        circ: E,
        resolution: u16,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        E: Circle<Scalar = f32>,
    {
        Transformable::new(
            self,
            color.map(),
            once(DrawType::Ellipse {
                center: circ.center().map(),
                radii: circ.radius().square(),
                resolution,
            }),
        )
    }
    pub fn ellipse<C, E>(
        &mut self,
        color: C,
        ellip: E,
        resolution: u16,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        E: Rectangle<Scalar = f32>,
    {
        Transformable::new(
            self,
            color.map(),
            once(DrawType::Ellipse {
                center: ellip.center().map(),
                radii: ellip.size().div(2.0).map(),
                resolution,
            }),
        )
    }
    pub fn polygon<'p, C, V, P>(&mut self, color: C, vertices: P) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        V: Vector2<Scalar = f32> + 'p,
        P: IntoIterator<Item = &'p V>,
    {
        Transformable::new(
            self,
            color.map(),
            once(DrawType::Polygon(
                vertices.into_iter().map(|v| v.map()).collect(),
            )),
        )
    }
    pub fn generic<V, I>(
        &mut self,
        primitive: PrimitiveType,
        vertices: V,
        indices: I,
    ) -> Transformable<'ctx, '_, T, R>
    where
        V: IntoIterator<Item = Vertex>,
        I: IntoIterator<Item = u16>,
    {
        Transformable::new(
            self,
            [1.0; 4],
            once(DrawType::Generic {
                vertices: vertices.into_iter().collect(),
                indices: Box::new(
                    IndexBuffer::new(
                        self.facade,
                        primitive,
                        &indices.into_iter().collect::<Vec<_>>(),
                    )
                    .unwrap(),
                ),
            }),
        )
    }
    pub fn line<C, P>(
        &mut self,
        color: C,
        endpoints: P,
        thickness: f32,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        P: Pair,
        P::Item: Vector2<Scalar = f32>,
    {
        let (a, b) = endpoints.to_pair();
        let a: Vec2 = a.map();
        let b: Vec2 = b.map();
        let perp = b
            .sub(a)
            .unit()
            .rotate_about(f32::TAU / 4.0, 0.0.square())
            .mul(thickness / 2.0);
        self.polygon(color, &[a.add(perp), b.add(perp), b.sub(perp), a.sub(perp)])
    }
}

/// Parameters for drawing rounded lines
#[derive(Debug, Clone, Copy)]
pub struct RoundLine {
    /// The thickness of the line
    pub thickness: f32,
    /// The resolution of the circle formed by each rounded end
    pub resolution: u16,
}

impl RoundLine {
    /// Create a new `RoundLine` with the given `thickness` and the
    /// default `resolution` of 20
    pub const fn new(thickness: f32) -> Self {
        RoundLine {
            thickness,
            resolution: 20,
        }
    }
    /// Set the `resolution`
    pub const fn resolution(self, resolution: u16) -> Self {
        RoundLine { resolution, ..self }
    }
}

impl From<f32> for RoundLine {
    fn from(thickness: f32) -> Self {
        RoundLine::new(thickness)
    }
}

impl<'ctx, T, R> Drawer<'ctx, T, R>
where
    T: Canvas,
    R: Resources,
{
    pub fn round_line<C, P, L>(
        &mut self,
        color: C,
        endpoints: P,
        rl: L,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        P: Pair,
        P::Item: Vector2<Scalar = f32>,
        L: Into<RoundLine>,
    {
        let (a, b) = endpoints.to_pair();
        let a: Vec2 = a.map();
        let b: Vec2 = b.map();
        let rl = rl.into();
        let diff = b.sub(a);
        let diff_unit = diff.unit();
        let radius = rl.thickness / 2.0;
        let perp = diff_unit.rotate(f32::TAU / 4.0).mul(radius);
        let length = diff.mag();
        let a_center = a.lerp(b, radius / length);
        let b_center = b.lerp(a, radius / length);
        let a_start = a_center.add(perp);
        let b_start = b_center.add(perp);
        let semi_res = rl.resolution / 2;
        let vertices: Vec<Vec2> = (0..=semi_res)
            .map(|i| {
                let angle = i as f32 / rl.resolution as f32 * f32::TAU;
                a_start.rotate_about(angle, a_center)
            })
            .chain((semi_res..=rl.resolution).map(|i| {
                let angle = i as f32 / rl.resolution as f32 * f32::TAU;
                b_start.rotate_about(angle, b_center)
            }))
            .collect();
        self.polygon(color, &vertices)
    }
    pub fn character<'drawer, C, L>(
        &'drawer mut self,
        color: C,
        ch: char,
        spec: L,
    ) -> Transformable<'ctx, 'drawer, T, R>
    where
        C: Color,
        L: Into<GlyphSpec<R::FontId>>,
    {
        let color: Col = color.map();
        let spec = spec.into();
        let scale_trans = GlyphSize::transform(&spec.size);
        if let Some(glyphs) = self.fonts.get(spec.font_id) {
            let glyph = glyphs.glyph(ch, spec.size.resolution).1.clone();
            Transformable::new(
                self,
                color,
                once(DrawType::Character {
                    vertices: glyph
                        .vertices
                        .into_iter()
                        .map(|v| v.transform(scale_trans))
                        .collect(),
                    indices: glyph.indices,
                    ch,
                    resolution: spec.size.resolution,
                }),
            )
        } else {
            Transformable::new(self, color, once(DrawType::Empty))
        }
    }
    pub fn text<C, L>(&mut self, color: C, string: &str, spec: L) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        L: Into<GlyphSpec<R::FontId>>,
    {
        use fontdue::layout::*;
        let color: Col = color.map();
        let spec = spec.into();
        let scale_trans = GlyphSize::transform(&spec.size);
        if let Some(glyphs) = self.fonts.get(spec.font_id) {
            let mut gps = Vec::new();
            Layout::new().layout_horizontal(
                &[glyphs.font()],
                &[&TextStyle::new(string, spec.size.resolution as f32, 0)],
                &LayoutSettings {
                    ..Default::default()
                },
                &mut gps,
            );
            let buffers: Vec<_> = gps
                .into_iter()
                .map(|gp| {
                    let (_, glyph) = &*glyphs.glyph(gp.key.c, spec.size.resolution);
                    let offset = [
                        gp.x,
                        -(spec.size.resolution as f32 + gp.y + gp.height as f32),
                    ];
                    (
                        glyph
                            .vertices
                            .iter()
                            .map(|v| v.add(offset).transform(scale_trans))
                            .collect(),
                        glyph.indices.clone(),
                        gp.key.c,
                    )
                })
                .collect();
            Transformable::new(
                self,
                color,
                buffers
                    .into_iter()
                    .map(|(vertices, indices, ch)| DrawType::Character {
                        vertices,
                        indices,
                        ch,
                        resolution: spec.size.resolution,
                    }),
            )
        } else {
            Transformable::new(self, color, once(DrawType::Empty))
        }
    }
}

enum DrawType {
    Empty,
    Rectangle(Rect),
    Ellipse {
        center: Vec2,
        radii: Vec2,
        resolution: u16,
    },
    Polygon(Vec<Vec2>),
    Generic {
        vertices: Vec<Vertex>,
        indices: Box<IndexBuffer<u16>>,
    },
    Character {
        vertices: Vec<Vec2>,
        indices: Rc<Vec<u16>>,
        ch: char,
        resolution: u32,
    },
}

#[derive(Debug, Clone, Copy)]
struct Border {
    color: Col,
    thickness: f32,
}

pub struct Transformable<'ctx, 'drawer, T, R>
where
    T: Canvas,
    R: Resources,
{
    drawer: &'drawer mut Drawer<'ctx, T, R>,
    tys: Rc<Vec<DrawType>>,
    color: Col,
    drawn: bool,
    transform: Trans,
    border: Option<Border>,
}

impl<'ctx, 'drawer, T, R> Transformable<'ctx, 'drawer, T, R>
where
    T: Canvas,
    R: Resources,
{
    pub fn color<'tfbl, C>(&'tfbl mut self, color: C) -> Transformable<'ctx, 'tfbl, T, R>
    where
        C: Color,
    {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            tys: Rc::clone(&self.tys),
            color: color.map(),
            transform: trans(),
            border: self.border,
            drawn: false,
        }
    }
    pub fn transform<'tfbl, D>(
        &'tfbl mut self,
        transformation: D,
    ) -> Transformable<'ctx, 'tfbl, T, R>
    where
        D: Fn(Trans) -> Trans,
    {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            tys: Rc::clone(&self.tys),
            color: self.color,
            transform: transformation(self.transform),
            border: self.border,
            drawn: false,
        }
    }
    pub fn border<'tfbl, C>(
        &'tfbl mut self,
        color: C,
        thickness: f32,
    ) -> Transformable<'ctx, 'tfbl, T, R>
    where
        C: Color,
    {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            tys: Rc::clone(&self.tys),
            color: self.color,
            transform: self.transform,
            border: Some(Border {
                color: color.map(),
                thickness,
            }),
            drawn: false,
        }
    }
    pub fn no_border<'tfbl>(&'tfbl mut self) -> Transformable<'ctx, 'tfbl, T, R> {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            tys: Rc::clone(&self.tys),
            color: self.color,
            transform: self.transform,
            border: None,
            drawn: false,
        }
    }
    pub fn draw(&mut self) {
        let uniforms = uniforms();
        let transform = self.drawer.camera.transform();
        for ty in &*self.tys {
            let mut vertices = self.unscaled_vertices(ty);
            for v in &mut vertices {
                v.pos = v.pos.transform(self.transform);
            }
            let border_vertices = self.border.as_ref().map(|_| vertices.clone());
            for v in &mut vertices {
                v.pos = v.pos.transform(transform);
            }
            let vertices = VertexBuffer::new(self.drawer.facade, &vertices).unwrap();
            let indices = self.drawer.indices.get(ty, self.drawer.facade);
            self.drawer
                .surface
                .draw(
                    &vertices,
                    indices,
                    self.drawer.program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
            if let Some((vertices, Border { color, thickness })) = border_vertices.zip(self.border)
            {
                if let Some(rect) = f32::Rect::bounding(vertices.iter().map(|v| v.pos)) {
                    let len = vertices.len() as u16;
                    let facade = self.drawer.facade;
                    let indices =
                        self.drawer
                            .indices
                            .get_or_insert(IndicesType::Border(len), || {
                                IndexBuffer::new(
                                    facade,
                                    PrimitiveType::TriangleStrip,
                                    &(0..(len * 2))
                                        .chain(once(0))
                                        .chain(once(1))
                                        .collect::<Vec<_>>(),
                                )
                                .unwrap()
                            });
                    let center = rect.center();
                    let radius = thickness / 2.0;
                    let vertices = vertices
                        .into_iter()
                        .flat_map(|v| {
                            let diff = v.pos.sub(center);
                            let length = diff.mag();
                            let unit = diff.unit();
                            once(Vertex {
                                pos: center.add(unit.mul(length + radius)).transform(transform),
                                color,
                            })
                            .chain(once(Vertex {
                                pos: center.add(unit.mul(length - radius)).transform(transform),
                                color,
                            }))
                        })
                        .collect::<Vec<_>>();
                    let vertices = VertexBuffer::new(self.drawer.facade, &vertices).unwrap();
                    self.drawer
                        .surface
                        .draw(
                            &vertices,
                            indices,
                            self.drawer.program,
                            &uniforms,
                            &Default::default(),
                        )
                        .unwrap();
                }
            }
        }
        self.drawn = true;
    }
    fn new<I>(drawer: &'drawer mut Drawer<'ctx, T, R>, color: Col, tys: I) -> Self
    where
        I: IntoIterator<Item = DrawType>,
    {
        Transformable {
            drawer,
            tys: Rc::new(tys.into_iter().collect()),
            color,
            transform: trans(),
            drawn: false,
            border: None,
        }
    }
    fn unscaled_vertices(&self, ty: &DrawType) -> Vec<Vertex> {
        match ty {
            DrawType::Empty => Vec::new(),
            DrawType::Rectangle(rect) => vec![
                Vertex {
                    pos: rect.top_left(),
                    color: self.color,
                },
                Vertex {
                    pos: rect.top_right(),
                    color: self.color,
                },
                Vertex {
                    pos: rect.bottom_right(),
                    color: self.color,
                },
                Vertex {
                    pos: rect.bottom_left(),
                    color: self.color,
                },
            ],
            DrawType::Ellipse {
                center,
                radii: [a, b],
                resolution,
            } => (0..*resolution)
                .map(|i| Vertex {
                    pos: center.add({
                        let angle = i as f32 / *resolution as f32 * f32::TAU;
                        let r = a * b
                            / ((b * angle.cos()).powf(2.0) + (a * angle.sin()).powf(2.0)).sqrt();
                        angle.angle_as_vector().mul(r)
                    }),
                    color: self.color,
                })
                .collect::<Vec<_>>(),
            DrawType::Polygon(ref vertices) => vertices
                .iter()
                .map(|&v| Vertex {
                    pos: v,
                    color: self.color,
                })
                .collect::<Vec<_>>(),
            DrawType::Generic { ref vertices, .. } => vertices.clone(),
            DrawType::Character { vertices, .. } => vertices
                .iter()
                .copied()
                .map(|pos| Vertex {
                    pos,
                    color: self.color,
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl<'ctx, 'drawer, T, R> Drop for Transformable<'ctx, 'drawer, T, R>
where
    T: Canvas,
    R: Resources,
{
    fn drop(&mut self) {
        if !self.drawn {
            self.draw();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum IndicesType {
    Empty,
    Rectangle,
    Ellipse(u16),
    Polygon(u16),
    Border(u16),
    Character { ch: char, resolution: u32 },
}

#[derive(Default)]
struct IndicesCache {
    map: HashMap<IndicesType, IndexBuffer<u16>>,
}

impl IndicesCache {
    #[allow(clippy::transmute_float_to_int)]
    fn get<'ctx, F>(&'ctx mut self, draw_type: &'ctx DrawType, facade: &F) -> &'ctx IndexBuffer<u16>
    where
        F: Facade,
    {
        match draw_type {
            DrawType::Empty => self.get_or_insert(IndicesType::Empty, || {
                IndexBuffer::empty(facade, PrimitiveType::Points, 0).unwrap()
            }),
            DrawType::Rectangle(_) => self.get_or_insert(IndicesType::Rectangle, || {
                IndexBuffer::new(facade, PrimitiveType::TrianglesList, &[0, 1, 2, 2, 3, 0]).unwrap()
            }),
            DrawType::Ellipse { resolution, .. } => {
                self.get_or_insert(IndicesType::Ellipse(*resolution), || {
                    IndexBuffer::new(
                        facade,
                        PrimitiveType::TrianglesList,
                        &(1..(*resolution - 2))
                            .flat_map(|n| once(0).chain(once(n)).chain(once(n + 1)))
                            .chain(
                                once(0)
                                    .chain(once(*resolution - 2))
                                    .chain(once(*resolution - 1)),
                            )
                            .collect::<Vec<_>>(),
                    )
                    .unwrap()
                })
            }
            DrawType::Polygon(vertices) => {
                let vertices = vertices.len() as u16;
                self.get_or_insert(IndicesType::Polygon(vertices), || {
                    IndexBuffer::new(
                        facade,
                        PrimitiveType::TrianglesList,
                        &(1..(vertices - 2))
                            .flat_map(|n| once(0).chain(once(n)).chain(once(n + 1)))
                            .chain(once(0).chain(once(vertices - 2)).chain(once(vertices - 1)))
                            .collect::<Vec<_>>(),
                    )
                    .unwrap()
                })
            }
            DrawType::Generic { indices, .. } => indices,
            DrawType::Character {
                indices,
                ch,
                resolution,
                ..
            } => self.get_or_insert(
                IndicesType::Character {
                    ch: *ch,
                    resolution: *resolution,
                },
                || IndexBuffer::new(facade, PrimitiveType::TrianglesList, indices).unwrap(),
            ),
        }
    }
    fn get_or_insert<G>(&mut self, it: IndicesType, g: G) -> &IndexBuffer<u16>
    where
        G: FnMut() -> IndexBuffer<u16>,
    {
        self.map.entry(it).or_insert_with(g)
    }
}

pub(crate) fn default_shaders<F>(facade: &F) -> Program
where
    F: Facade,
{
    program!(facade,
        140 => {
            vertex: "
                #version 140

                uniform mat4 matrix;

                in vec2 pos;
                in vec4 color;

                out vec4 vColor;

                void main() {
                    gl_Position = vec4(pos, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 140
                in vec4 vColor;
                out vec4 f_color;

                void main() {
                    f_color = vColor;
                }
            "
        },

        110 => {
            vertex: "
                #version 110

                uniform mat4 matrix;

                attribute vec2 pos;
                attribute vec4 color;

                varying vec4 vColor;

                void main() {
                    gl_Position = vec4(pos, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 110
                varying vec4 vColor;

                void main() {
                    gl_FragColor = vColor;
                }
            ",
        },

        100 => {
            vertex: "
                #version 100

                uniform lowp mat4 matrix;

                attribute lowp vec2 pos;
                attribute lowp vec4 color;

                varying lowp vec4 vColor;

                void main() {
                    gl_Position = vec4(pos, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 100
                varying lowp vec4 vColor;

                void main() {
                    gl_FragColor = vColor;
                }
            ",
        },
    )
    .unwrap()
}
