#version 330

uniform float x_off;
uniform mat4 matrix;
uniform mat4 perspective;
uniform vec2 xy_off;

in vec3 position;
in vec3 velocity;

out float colours;

void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    colours = position.z;
}