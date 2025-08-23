#version 330

layout (triangles) in;
layout(triangle_strip, max_vertices = 3) out; 

uniform mat4 perspective;

in vec3 geoColours[];

out float radius;
out vec4 centre;
out vec3 colour;
out vec4 fragCoord;


void main() {   

    centre =  (gl_in[0].gl_Position + gl_in[1].gl_Position + gl_in[2].gl_Position)/3.0 ; 
    radius = (gl_in[0].gl_Position.y - centre.y)/2;
    colour = geoColours[0];
    gl_Position = gl_in[0].gl_Position;
    fragCoord =  gl_Position;
    EmitVertex();
    gl_Position = gl_in[1].gl_Position;
    fragCoord = gl_Position;
    EmitVertex();
    gl_Position = gl_in[2].gl_Position;
    fragCoord = gl_Position;
    EmitVertex();
    
    EndPrimitive();
} 
