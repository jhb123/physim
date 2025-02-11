#version 330

in vec3 position;
uniform float x_off;
uniform mat4 matrix;       // new
uniform mat4 perspective;       // new
uniform vec2 xy_off;       // new

out float colours;
void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    colours = position.z;
}