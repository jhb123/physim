#version 330

uniform vec2 resolution;

in vec4 centre;
in vec4 fragCoord;
in float radius;
in float colour;

out vec4 FragColor;

void main() {
    vec4 f = fragCoord;
    vec4 c = centre;
    f.x *= resolution[0]/resolution[1];
    c.x *= resolution[0]/resolution[1];


    if ( distance(f.xy,c.xy) > radius ){
        discard;
    } else {
        FragColor = vec4(1.0-colour,1.0,colour,0.5);
    }
}
