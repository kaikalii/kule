use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt,
    iter::once,
    rc::Rc,
};

use glium::{backend::*, *};
use vector2math::*;

use crate::{Col, Color, Fonts, GlyphSize, GlyphSpec, Rect, Resources, Trans, Vec2};

pub use index::PrimitiveType;

#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    pub pos: Vec2,
}

implement_vertex!(Vertex, pos);

fn extend_transform(trans: Trans) -> [[f32; 3]; 3] {
    [trans[0], trans[1], [0.0, 0.0, 1.0]]
}

/// A scene camera
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// The center of the scene
    pub center: Vec2,
    /// The zoom factor
    pub zoom: f32,
    pub(crate) window_size: Vec2,
}

impl Camera {
    /// Get the size of the window
    pub fn window_size(self) -> Vec2 {
        self.window_size
    }
    /// Set the center
    pub fn with_center(self, center: Vec2) -> Self {
        Camera { center, ..self }
    }
    /// Set the zoom
    pub fn with_zoom(self, zoom: f32) -> Self {
        Camera { zoom, ..self }
    }
    /// Multiply the zoom by some factor
    pub fn zoom_by(self, by: f32) -> Self {
        Camera {
            zoom: self.zoom * by,
            ..self
        }
    }
    /// Keep the zoom within some bounds
    pub fn bound_zoom(self, min: f32, max: f32) -> Self {
        Camera {
            zoom: self.zoom.max(min).min(max),
            ..self
        }
    }
    /// Translate the center
    pub fn translate(self, offset: Vec2) -> Self {
        Camera {
            center: self.center.add(offset),
            ..self
        }
    }
    /// Convert a vector from window space to world space
    pub fn pos_to_coords(self, pos: Vec2) -> Vec2 {
        pos.sub(self.window_size.div(2.0))
            .div(self.zoom)
            .add(self.center)
    }
    /// Convert a vector frrom world space to window space
    pub fn coords_to_pos(self, coords: Vec2) -> Vec2 {
        coords
            .sub(self.center)
            .div(2.0)
            .mul(self.zoom)
            .add(self.window_size.div(2.0))
    }
    /// Get the rectangle that bounds the view
    pub fn view_rect(self) -> Rect {
        Rect::centered(self.center, self.window_size.div(self.zoom))
    }
    fn transform(&self) -> Trans {
        Trans::new_translate(self.center.neg())
            .scale([self.zoom; 2].mul2([1.0, -1.0]))
            .scale::<Vec2>(self.window_size.map_with(|d| 1.0 / d))
            .zoom(2.0)
    }
}

type Vertices = VertexBuffer<Vertex>;
type Indices = IndexBuffer<u16>;
type MeshMap<R> = HashMap<DrawType<R>, (Vertices, Indices)>;

/**
A cache for gpu geometry

Most simple geometries are cached automatically. However, irregular polygons are
not cached by default. If a shape is drawn using a `Drawer::cached_*` method,
this cache can be accessed to remove old versions of cached meshes.
*/
pub struct MeshCache<R>(Rc<RefCell<MeshMap<R>>>)
where
    R: Resources;

impl<R> Default for MeshCache<R>
where
    R: Resources,
{
    fn default() -> Self {
        MeshCache(Default::default())
    }
}

impl<R> MeshCache<R>
where
    R: Resources,
{
    pub(crate) fn insert(
        &self,
        draw_type: DrawType<R>,
        vertices: VertexBuffer<Vertex>,
        indices: IndexBuffer<u16>,
    ) {
        self.0.borrow_mut().insert(draw_type, (vertices, indices));
    }
    pub(crate) fn contains(&self, draw_type: &DrawType<R>) -> bool {
        self.0.borrow().contains_key(draw_type)
    }
    pub(crate) fn get(&self, draw_type: &DrawType<R>) -> Option<(Ref<Vertices>, Ref<Indices>)> {
        let map = self.0.borrow();
        if map.contains_key(draw_type) {
            Some(Ref::map_split(self.0.borrow(), |map| {
                let (vertices, indices) = &map[draw_type];
                (vertices, indices)
            }))
        } else {
            None
        }
    }
    /// Check if the cache contains a mesh
    pub fn contains_mesh(&self, mesh_id: R::MeshId) -> bool {
        self.contains(&DrawType::Irregular(Some(mesh_id)))
    }
    /// Clear only the irregular cached meshes
    pub fn clear_meshes(&self) {
        self.0
            .borrow_mut()
            .retain(|draw_type, _| !matches!(draw_type, DrawType::Irregular(_)));
    }
    /// Clear all meshes
    pub fn clear_all(&self) {
        self.0.borrow_mut().clear();
    }
    /// Move a manually cached mesh
    pub fn remove_mesh(&self, mesh_id: R::MeshId) {
        self.0
            .borrow_mut()
            .remove(&DrawType::Irregular(Some(mesh_id)));
    }
}

/// Trait for defining drawing types
pub trait Canvas {
    /// The gpu facade
    type Facade: Facade;
    /// The surface being drawn to
    type Surface: Surface;
}

/// The canvas used for drawing to a window
pub struct WindowCanvas;

impl Canvas for WindowCanvas {
    type Facade = Display;
    type Surface = Frame;
}

/// The primary struct for drawing 2d geometry
pub struct Drawer<'ctx, T = WindowCanvas, R = ()>
where
    T: Canvas,
    R: Resources,
{
    surface: &'ctx mut T::Surface,
    facade: &'ctx T::Facade,
    program: &'ctx Program,
    /// The fonts
    pub fonts: &'ctx Fonts<R::FontId>,
    /// The mesh cache
    pub meshes: &'ctx MeshCache<R>,
    /// The scene camera
    pub camera: Camera,
    /// The draw parameters
    pub draw_params: DrawParameters<'ctx>,
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
        meshes: &'ctx MeshCache<R>,
        camera: Camera,
    ) -> Self {
        Drawer {
            surface,
            facade,
            program,
            fonts,
            camera,
            meshes,
            draw_params: DrawParameters {
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        }
    }
    /**
    Temporarily use a different camera for drawing

    The camera is changed, the `draw` closure is called, and then
    the camera is returned to its original state.
    */
    pub fn with_camera<C, F, S>(&mut self, camera: C, draw: F) -> S
    where
        C: FnOnce(Camera) -> Camera,
        F: FnOnce(&mut Self) -> S,
    {
        let base_camera = self.camera;
        self.camera = camera(base_camera);
        let res = draw(self);
        self.camera = base_camera;
        res
    }
    /**
    Temporarily use an absolute camera for drawing

    The camera is changed, the `draw` closure is called, and then
    the camera is returned to its original state.

    The camera used is one where window space and world space are the same
    */
    pub fn with_absolute_camera<F, S>(&mut self, draw: F) -> S
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
            draw,
        )
    }
    /// Clear the surface with a color
    ///
    /// This clears the depth and stencil buffers as well
    pub fn clear<C>(&mut self, color: C)
    where
        C: Color,
    {
        self.surface.clear_all(color.map(), 0.0, 0)
    }
    /// Draw a rectangle
    pub fn rectangle<C, E>(&mut self, color: C, rect: E) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        E: Rectangle<Scalar = f32>,
    {
        let rect: [f32; 4] = rect.map();
        Transformable::new(
            self,
            color.map(),
            DrawType::Regular(4),
            Trans::identity()
                .scale(rect.size().mul(0.5 * std::f32::consts::SQRT_2))
                .translate(rect.center()),
        )
    }
    /// Draw a circle
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
            DrawType::Regular(resolution),
            Trans::identity()
                .zoom(circ.radius())
                .translate(circ.center()),
        )
    }
    /// Draw an ellipse
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
            DrawType::Regular(resolution),
            Trans::identity()
                .scale(ellip.size().div(2.0))
                .translate(ellip.center()),
        )
    }
    /// Draw a polygon
    pub fn polygon<'p, C, V, P>(&mut self, color: C, vertices: P) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        V: Vector2<Scalar = f32> + 'p,
        P: IntoIterator<Item = &'p V>,
    {
        self.optionally_cached_polygon(None, color, vertices)
    }
    /// Draw a polygon with cached geometry
    pub fn cached_polygon<'p, C, V, P>(
        &mut self,
        mesh_id: R::MeshId,
        color: C,
        vertices: P,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        V: Vector2<Scalar = f32> + 'p,
        P: IntoIterator<Item = &'p V>,
    {
        self.optionally_cached_polygon(Some(mesh_id), color, vertices)
    }
    fn optionally_cached_polygon<'p, C, V, P>(
        &mut self,
        mesh_id: Option<R::MeshId>,
        color: C,
        vertices: P,
    ) -> Transformable<'ctx, '_, T, R>
    where
        C: Color,
        V: Vector2<Scalar = f32> + 'p,
        P: IntoIterator<Item = &'p V>,
    {
        let vertices = VertexBuffer::new(
            self.facade,
            &vertices
                .into_iter()
                .map(|v| Vertex { pos: v.map() })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let len = vertices.len() as u16;
        let indices = IndexBuffer::new(
            self.facade,
            PrimitiveType::TrianglesList,
            &(1..(len - 2))
                .flat_map(|n| once(0).chain(once(n)).chain(once(n + 1)))
                .chain(once(0).chain(once(len - 2)).chain(once(len - 1)))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        self.meshes
            .insert(DrawType::Irregular(mesh_id), vertices, indices);
        Transformable::new(
            self,
            color.map(),
            DrawType::Irregular(mesh_id),
            Trans::identity(),
        )
    }
    /// Draw a line
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
        let diff = b.sub(a);
        let length = diff.mag();
        let midpoint = a.lerp(b, 0.5);
        let rot = diff.atan();
        Transformable::new(
            self,
            color.map(),
            DrawType::Regular(4),
            Trans::identity()
                .scale([length, thickness].mul(0.5 * std::f32::consts::SQRT_2))
                .rotate(rot)
                .translate(midpoint),
        )
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
    /// default `resolution` of `20`
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
    /// Draw a line with rounded ends
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
        self.optionally_cached_round_line(None, color, endpoints, rl)
    }
    /// Draw a line with rounded ends with cached geometry
    pub fn cached_round_line<C, P, L>(
        &mut self,
        mesh_id: R::MeshId,
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
        self.optionally_cached_round_line(Some(mesh_id), color, endpoints, rl)
    }
    fn optionally_cached_round_line<C, P, L>(
        &mut self,
        mesh_id: Option<R::MeshId>,
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
        self.optionally_cached_polygon(mesh_id, color, &vertices)
    }
    /// Draw a single character
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
        Transformable::new(
            self,
            color,
            DrawType::Character {
                ch,
                resolution: spec.size.resolution,
                font_id: spec.font_id,
            },
            scale_trans,
        )
    }
    /// Draw a string of text
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
            let offset_chars: Vec<_> = gps
                .into_iter()
                .map(|gp| {
                    let offset = [
                        gp.x,
                        -(spec.size.resolution as f32 + gp.y + gp.height as f32),
                    ];
                    (offset, gp.key.c)
                })
                .collect();
            Transformable::multi(
                self,
                color,
                offset_chars.into_iter().map(|(offset, ch)| DrawItem {
                    ty: DrawType::Character {
                        ch,
                        resolution: spec.size.resolution,
                        font_id: spec.font_id,
                    },
                    transform: Trans::new_translate(offset).then(scale_trans),
                    color: None,
                }),
                Trans::identity(),
            )
        } else {
            Transformable::new(self, color, DrawType::Empty, Trans::identity())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DrawType<R>
where
    R: Resources,
{
    Empty,
    Regular(u16),
    Irregular(Option<R::MeshId>),
    Character {
        ch: char,
        resolution: u32,
        font_id: R::FontId,
    },
}

impl<R> DrawType<R>
where
    R: Resources,
{
    fn vertices_indices<F>(
        self,
        facade: &F,
        fonts: &Fonts<R::FontId>,
    ) -> (VertexBuffer<Vertex>, IndexBuffer<u16>)
    where
        F: Facade,
    {
        match self {
            DrawType::Empty => (
                VertexBuffer::empty(facade, 0).unwrap(),
                IndexBuffer::empty(facade, PrimitiveType::Points, 0).unwrap(),
            ),
            DrawType::Regular(n) => {
                let angle_offset = f32::TAU / n as f32 / 2.0;
                let vertices: Vec<Vertex> = (0..n)
                    .map(|i| (i as f32 / n as f32 * f32::TAU + angle_offset).angle_as_vector())
                    .map(|pos| Vertex { pos })
                    .collect();
                let indices: Vec<u16> = (1..(n - 2))
                    .flat_map(|n| once(0).chain(once(n)).chain(once(n + 1)))
                    .chain(once(0).chain(once(n - 2)).chain(once(n - 1)))
                    .collect();
                (
                    VertexBuffer::new(facade, &vertices).unwrap(),
                    IndexBuffer::new(facade, PrimitiveType::TrianglesList, &indices).unwrap(),
                )
            }
            DrawType::Irregular(_) => {
                panic!("called DrawType::vertices_indices on DrawType::Irregular")
            }
            DrawType::Character {
                ch,
                resolution,
                font_id,
            } => {
                let (_, geometry) = &*fonts[font_id].glyph(ch, resolution);
                let vertices = VertexBuffer::new(
                    facade,
                    &geometry
                        .vertices
                        .iter()
                        .map(|&pos| Vertex { pos })
                        .collect::<Vec<_>>(),
                )
                .unwrap();
                let indices =
                    IndexBuffer::new(facade, PrimitiveType::TrianglesList, &geometry.indices)
                        .unwrap();
                (vertices, indices)
            }
        }
    }
}

impl<R> fmt::Debug for DrawType<R>
where
    R: Resources,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DrawType::Empty => write!(f, "Empty"),
            DrawType::Regular(n) => write!(f, "{} sides", n),
            DrawType::Irregular(None) => write!(f, "Uncached"),
            DrawType::Irregular(Some(mesh_id)) => write!(f, "Cached ({:?})", mesh_id),
            DrawType::Character {
                ch,
                resolution,
                font_id,
            } => write!(f, "'{}' at {}px with {:?}", ch, resolution, font_id),
        }
    }
}

struct DrawItem<R>
where
    R: Resources,
{
    ty: DrawType<R>,
    transform: Trans,
    color: Option<Col>,
}

#[derive(Debug, Clone, Copy)]
struct Border {
    color: Col,
    thickness: f32,
}

/**
A planned draw command

Drawing can be done manually with the `draw` method, but this method is
also called automatically when the struct is dropped if it has not been
called already.
*/
pub struct Transformable<'ctx, 'drawer, T, R>
where
    T: Canvas,
    R: Resources,
{
    drawer: &'drawer mut Drawer<'ctx, T, R>,
    items: Rc<Vec<DrawItem<R>>>,
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
    /// Change the color
    pub fn color<'tfbl, C>(&'tfbl mut self, color: C) -> Transformable<'ctx, 'tfbl, T, R>
    where
        C: Color,
    {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            items: Rc::clone(&self.items),
            color: color.map(),
            transform: self.transform,
            drawn: false,
            border: self.border,
        }
    }
    /// Apply a transformation
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
            items: Rc::clone(&self.items),
            color: self.color,
            transform: transformation(self.transform),
            drawn: false,
            border: self.border,
        }
    }
    /// Apply a translation
    pub fn translate<'tfbl>(&'tfbl mut self, offset: Vec2) -> Transformable<'ctx, 'tfbl, T, R> {
        self.transform(|t| t.translate(offset))
    }
    /// Set a border
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
            items: Rc::clone(&self.items),
            color: self.color,
            transform: self.transform,
            drawn: false,
            border: Some(Border {
                color: color.map(),
                thickness,
            }),
        }
    }
    /// Remove the border
    pub fn no_border<'tfbl>(&'tfbl mut self) -> Transformable<'ctx, 'tfbl, T, R> {
        self.drawn = true;
        Transformable {
            drawer: self.drawer,
            items: Rc::clone(&self.items),
            color: self.color,
            transform: self.transform,
            drawn: false,
            border: None,
        }
    }
    /**
    Execute the draw command

    This is usually called automatically
    */
    pub fn draw(&mut self) {
        let camera_transform = self.drawer.camera.transform();
        for item in self.items.iter() {
            let Drawer {
                meshes,
                facade,
                fonts,
                surface,
                program,
                draw_params,
                ..
            } = &mut self.drawer;
            if !meshes.contains(&item.ty) {
                let (vertices, indices) = item.ty.vertices_indices(*facade, fonts);
                meshes.insert(item.ty, vertices, indices);
            }
            let (vertices, indices) = meshes.get(&item.ty).unwrap();
            let world_transform = item.transform.then(self.transform);
            let full_transform = world_transform.then(camera_transform);
            let uniforms = uniform! {
                transform: extend_transform(full_transform),
                color: item.color.unwrap_or(self.color)
            };
            surface
                .draw(&*vertices, &*indices, program, &uniforms, draw_params)
                .unwrap();
            // Draw border
            if let Some(border) = self.border {
                let bounding_rect = Rect::bounding(
                    vertices
                        .read()
                        .unwrap()
                        .iter()
                        .map(|v| v.pos.transform(world_transform)),
                );
                if let Some(bounding_rect) = bounding_rect {
                    let center = bounding_rect.center();
                    let size = bounding_rect.size();
                    let scale = size.add([border.thickness; 2]).div2(size);
                    // Draw stencil
                    let border_inner_transform = world_transform
                        .translate(center.neg())
                        .scale([1.0; 2].div2(scale))
                        .translate(center)
                        .then(camera_transform);
                    let uniforms = uniform! {
                        transform: extend_transform(border_inner_transform),
                        color: [0f32; 4]
                    };
                    let draw_params = DrawParameters {
                        stencil: draw_parameters::Stencil {
                            reference_value_clockwise: 1,
                            reference_value_counter_clockwise: 1,
                            write_mask_clockwise: 0xffffffff,
                            write_mask_counter_clockwise: 0xffffffff,
                            depth_pass_operation_clockwise: StencilOperation::Replace,
                            depth_pass_operation_counter_clockwise: StencilOperation::Replace,
                            ..Default::default()
                        },
                        ..draw_params.clone()
                    };
                    surface
                        .draw(&*vertices, &*indices, program, &uniforms, &draw_params)
                        .unwrap();
                    // Draw border
                    let border_outer_transform = world_transform
                        .translate(center.neg())
                        .scale(scale)
                        .translate(center)
                        .then(camera_transform);
                    let uniforms = uniform! {
                        transform: extend_transform(border_outer_transform),
                        color: border.color
                    };
                    let draw_params = DrawParameters {
                        stencil: draw_parameters::Stencil {
                            reference_value_clockwise: 1,
                            reference_value_counter_clockwise: 1,
                            test_clockwise: StencilTest::IfNotEqual { mask: 0xffffffff },
                            test_counter_clockwise: StencilTest::IfNotEqual { mask: 0xffffffff },
                            ..Default::default()
                        },
                        ..draw_params.clone()
                    };
                    surface
                        .draw(&*vertices, &*indices, program, &uniforms, &draw_params)
                        .unwrap();
                    surface.clear_stencil(0);
                }
            }
        }
        self.drawn = true;
    }
    fn new(
        drawer: &'drawer mut Drawer<'ctx, T, R>,
        color: Col,
        ty: DrawType<R>,
        transform: Trans,
    ) -> Self {
        Transformable::multi(
            drawer,
            color,
            once(DrawItem {
                ty,
                transform: Trans::identity(),
                color: None,
            }),
            transform,
        )
    }
    fn multi<I>(
        drawer: &'drawer mut Drawer<'ctx, T, R>,
        color: Col,
        items: I,
        transform: Trans,
    ) -> Self
    where
        I: IntoIterator<Item = DrawItem<R>>,
    {
        Transformable {
            drawer,
            items: Rc::new(items.into_iter().collect()),
            color,
            transform,
            drawn: false,
            border: None,
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

pub(crate) fn default_shaders<F>(facade: &F) -> Program
where
    F: Facade,
{
    Program::new(
        facade,
        program::SourceCode {
            vertex_shader: include_str!("shaders/vertex.vert"),
            fragment_shader: include_str!("shaders/fragment.frag"),
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
        },
    )
    .unwrap_or_else(|e| panic!("{}", e))
}
