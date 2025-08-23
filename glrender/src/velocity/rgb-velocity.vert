#version 330

uniform float x_off;
uniform mat4 matrix;
uniform mat4 perspective;
uniform vec2 xy_off;

in vec3 position;
in vec3 velocity;

out vec3 geoColours;

vec3 hsv2rgb(vec3 c) {
    // c.x is hue
    // c.y is sat
    // c.z is val

    // 6.0 is full range of channel
    // vec3(0.0, 4.0, 2.0) is a phase shift for each channel
    // mod + abs makes a sawtooth pattern for each channel
    // you get a vec3 for rgb vals
    vec3 rgb = clamp(
        abs(mod(c.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0,
        0.0,
        1.0
    );
    // interpolate saturation and set val.
    return c.z * mix(vec3(1.0), rgb, c.y);
}


void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    
    float speed = length(velocity);
    float hue = fract(speed*0.5);
    float sat = 0.8;
    float val = 1.0;

    geoColours = hsv2rgb(vec3(hue, sat, val));
}
