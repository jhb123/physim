#version 330

uniform float x_off;
uniform mat4 matrix;
uniform mat4 perspective;
uniform vec2 xy_off;

in vec3 position;
in vec3 velocity;

out vec3 geoColours;

void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    float v = sin(length(velocity));
    float v2 = cos(length(velocity));

    geoColours = vec3(1-v,1-v2,1.0);
}
