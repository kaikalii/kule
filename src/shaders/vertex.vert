#version 140

uniform mat4 matrix;

in vec2 pos;
in vec4 color;

out vec4 vertex_color;

void main() {
  gl_Position = vec4(pos, 0.0, 1.0) * matrix;
  vertex_color = color;
}