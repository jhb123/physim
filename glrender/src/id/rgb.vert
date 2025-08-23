#version 330

uniform float x_off;
uniform mat4 matrix;
uniform mat4 perspective;
uniform vec2 xy_off;
uniform int frame_num; 

in vec3 position;
in vec3 velocity;
in int id;

out vec3 geoColours;

vec3 idToColor(int id) {
    vec3 colors[12];
    colors[0]  = vec3(1.0, 0.0, 0.0);   // red
    colors[1]  = vec3(0.0, 1.0, 0.0);   // green
    colors[2]  = vec3(0.2, 0.2, 1.0);   // blue
    colors[3]  = vec3(1.0, 1.0, 0.0);   // yellow
    colors[4]  = vec3(1.0, 0.0, 1.0);   // magenta
    colors[5]  = vec3(0.0, 1.0, 1.0);   // cyan
    colors[6]  = vec3(1.0, 0.5, 0.0);   // orange
    colors[7]  = vec3(1.0, 0.0, 0.5);   // pink
    colors[8]  = vec3(0.5, 0.0, 1.0);   // purple
    colors[9]  = vec3(0.0, 0.5, 1.0);   // sky blue
    colors[10] = vec3(0.5, 1.0, 0.0);   // lime
    colors[11] = vec3(0.0, 1.0, 0.5);   // teal

    int index = id % 12; // wrap around the 12-color palette
    return colors[index];
}


void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    geoColours = idToColor(id);
}
