#version 140

uniform mat3 transform;
uniform vec4 color;

in vec2 pos;

out vec4 vertex_color;

void main() {
  gl_Position = vec4(vec3(pos, 1.0) * transform, 1.0);
  vertex_color = color;
}