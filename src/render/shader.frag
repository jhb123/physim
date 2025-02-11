#version 330

// in vec2 fragCoord;
in vec4 centre;
in vec4 fragCoord;
in float radius;
out vec4 FragColor;
uniform vec2 resolution;
in float colour;
void main() {
    vec4 f = fragCoord;
    vec4 c = centre;
    f.x *= resolution[0]/resolution[1];
    c.x *= resolution[0]/resolution[1];


    if ( distance(f.xy,c.xy) > radius ){
        discard;
    } else {
        FragColor = vec4(1.0-colour,1.0,colour,1.0);
    }
}