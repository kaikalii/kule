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
            [0.0, 0.0, 0.0, 1.0f32]
        ]
    }
}

pub struct Drawer<'a, S, F> {
    surface: &'a mut S,
    facade: &'a F,
    program: &'a Program,
    indices: IndicesCache,
}

impl<'a, S, F> Drawer<'a, S, F>
where
    S: Surface,
    F: Facade,
{
    pub(crate) fn new(surface: &'a mut S, facade: &'a F, program: &'a Program) -> Self {
        Drawer {
            surface,
            facade,
            program,
            indices: Default::default(),
        }
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
        let rect: [f32; 4] = rect.map();
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
