use std::collections::HashMap;

use glium::{backend::*, uniforms::*, *};
use vector2math::*;

use crate::{Col, Color, Vec2};

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
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            center: [0.0; 2],
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn with_center(self, center: Vec2) -> Self {
        Camera { center, ..self }
    }
    pub fn with_zoom(self, zoom: f32) -> Self {
        Camera { zoom, ..self }
    }
    pub fn map_center<F>(self, f: F) -> Self
    where
        F: FnOnce(Vec2) -> Vec2,
    {
        Camera {
            center: f(self.center),
            ..self
        }
    }
    pub fn map_zoom<F>(self, f: F) -> Self
    where
        F: FnOnce(f32) -> f32,
    {
        Camera {
            zoom: f(self.zoom),
            ..self
        }
    }
    fn transform_rect<R>(&self, window_size: Vec2, rect: R) -> R
    where
        R: Rectangle<Scalar = f32>,
    {
        R::new(
            self.transform_point(window_size, rect.top_left()),
            rect.size().div2(window_size).mul(self.zoom),
        )
    }
    fn transform_point<V>(&self, window_size: Vec2, p: V) -> V
    where
        V: Vector2<Scalar = f32>,
    {
        p.sub(self.center).mul(self.zoom).div2(window_size)
    }
}

pub struct Drawer<'a, S, F> {
    surface: &'a mut S,
    facade: &'a F,
    program: &'a Program,
    camera: Camera,
    window_size: Vec2,
    indices: IndicesCache,
}

impl<'a, S, F> Drawer<'a, S, F>
where
    S: Surface,
    F: Facade,
{
    pub(crate) fn new(
        surface: &'a mut S,
        facade: &'a F,
        program: &'a Program,
        camera: Camera,
        window_size: Vec2,
    ) -> Self {
        Drawer {
            surface,
            facade,
            program,
            camera,
            window_size,
            indices: Default::default(),
        }
    }
    pub fn with_camera<C, G, R>(&mut self, camera: C, g: G) -> R
    where
        C: FnOnce(Camera) -> Camera,
        G: FnOnce() -> R,
    {
        let base_camera = self.camera;
        self.camera = camera(base_camera);
        let res = g();
        self.camera = base_camera;
        res
    }
    pub fn with_absolute_camera<G, R>(&mut self, g: G) -> R
    where
        G: FnOnce() -> R,
    {
        let base_camera = self.camera;
        self.camera = Camera {
            center: self.window_size.div(2.0),
            zoom: 1.0,
        };
        let res = g();
        self.camera = base_camera;
        res
    }
    pub fn clear<C>(&mut self, color: C)
    where
        C: Color,
    {
        self.surface
            .clear_color(color.r(), color.g(), color.b(), color.alpha())
    }
    pub fn rectangle<C, R>(&mut self, color: C, rect: R)
    where
        C: Color,
        R: Rectangle<Scalar = f32>,
    {
        let color: [f32; 4] = color.map();
        let rect: [f32; 4] = self.camera.transform_rect(self.window_size, rect).map();
        let vertices = VertexBuffer::new(
            self.facade,
            &[
                Vertex {
                    pos: rect.top_left(),
                    color,
                },
                Vertex {
                    pos: rect.top_right(),
                    color,
                },
                Vertex {
                    pos: rect.bottom_right(),
                    color,
                },
                Vertex {
                    pos: rect.bottom_left(),
                    color,
                },
            ],
        )
        .unwrap();
        self.surface
            .draw(
                &vertices,
                self.indices.rectangle(self.facade),
                self.program,
                &uniforms(),
                &Default::default(),
            )
            .unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum IndicesType {
    Rectangle,
}

#[derive(Default)]
struct IndicesCache {
    map: HashMap<IndicesType, IndexBuffer<u16>>,
}

impl IndicesCache {
    fn rectangle<F>(&mut self, facade: &F) -> &IndexBuffer<u16>
    where
        F: Facade,
    {
        self.map.entry(IndicesType::Rectangle).or_insert_with(|| {
            IndexBuffer::new(
                facade,
                index::PrimitiveType::TrianglesList,
                &[0, 1, 2, 2, 3, 0],
            )
            .unwrap()
        })
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
