#version 330

uniform float x_off;
uniform mat4 matrix;
uniform mat4 perspective;
uniform vec2 xy_off;
uniform int frame_num; 

in vec3 position;
in vec3 velocity;

out vec3 geoColours;

void main() {
    vec3 pos = position;
    pos.xy = pos.xy + xy_off;
    gl_Position = perspective*matrix*vec4(pos.xyz, 1.0);
    
    float c = 0.7*fract( 10.0*(pos.x + pos.y + pos.z) + frame_num );


    geoColours = vec3(1-c,1-c,1-c);
}
