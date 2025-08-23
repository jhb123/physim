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
    float t = 0.5*(sin(length(2.0*velocity)) + 1.0);

    vec3 orange = vec3(1.0, 0.5, 0.0);
    vec3 blue   = vec3(0.0, 0.2, 1.0);

    geoColours = mix(orange, blue, t);
}
