#[macro_use]
extern crate glium;

use glium::{
    winit::{
        event::{Event, WindowEvent},
        event_loop::EventLoop,
    },
    Surface,
};
use log::info;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
struct Circle {
    centre: [f32; 2],
    radius: f32,
    verticies: [Vertex; 3],
}

impl Circle {
    fn new(centre: [f32; 2], radius: f32) -> Self {
        let verticies = [
            Vertex {
                position: [centre[0], centre[1] + radius],
            },
            Vertex {
                position: [
                    centre[0] + radius * f32::sqrt(3.0) * 0.5,
                    centre[1] - 0.5 * radius,
                ],
            },
            Vertex {
                position: [
                    centre[0] - radius * f32::sqrt(3.0) * 0.5,
                    centre[1] - 0.5 * radius,
                ],
            },
        ];

        Self {
            centre: centre,
            radius: radius,
            verticies: verticies,
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::builder().build().expect("event loop building");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("PhySim Renderer")
        .build(&event_loop);
    let circle = Circle::new([-0.4, 0.1], 0.9);
    let shape = circle.verticies;
    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 330

        in vec2 position;
        uniform float x_off;

        uniform float aspectRatio;
        // out vec2 fragCoord; // Passing to Fragment Shader

        void main() {
            vec2 pos = position;
            pos.x += x_off;
            // fragCoord = position;
            gl_Position = vec4(pos.x*aspectRatio, pos.y , 0.0, 1.0);
        }
    "#;

    let geometry_shader_src = r#"
        #version 330

        layout (triangles) in;
        layout(triangle_strip, max_vertices = 3) out; 
        out float radius;
        out vec4 centre;
        out vec2 fragCoord;
        uniform float aspectRatio;

        void main() {    
            centre = (gl_in[0].gl_Position + gl_in[1].gl_Position + gl_in[2].gl_Position)/3.0 ; 
            centre.x /= aspectRatio;
            // centre = tricentre;

            radius = (gl_in[0].gl_Position.y - centre.y)/2;

            gl_Position = gl_in[0].gl_Position;
            fragCoord = vec2(gl_Position.x/aspectRatio, gl_Position.y);
            EmitVertex();
            gl_Position = gl_in[1].gl_Position;
            fragCoord = vec2(gl_Position.x/aspectRatio, gl_Position.y);
            EmitVertex();
            gl_Position = gl_in[2].gl_Position;
            fragCoord = vec2(gl_Position.x/aspectRatio, gl_Position.y);
            EmitVertex();
            
            EndPrimitive();
        } 
    "#;

    let fragment_shader_src = r#"
        #version 330

        in vec4 centre;
        in vec2 fragCoord;
        in float radius;
        out vec4 FragColor;
        uniform float aspectRatio;

        void main() {

            if (distance(fragCoord.xy, centre.xy) > radius) {
                discard;
            } else {
                FragColor = vec4(1.0, 1.0, 1.0, 1.0);
            }

        }
    "#;

    let program = glium::Program::from_source(
        &display,
        vertex_shader_src,
        fragment_shader_src,
        Some(geometry_shader_src),
    )
    .unwrap();

    let mut t: f32 = 0.0;

    // this avoids a lot of boiler plate.
    #[warn(deprecated)]
    let _ = event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => window_target.exit(),
                WindowEvent::Resized(window_size) => {
                    display.resize(window_size.into());
                },
                WindowEvent::RedrawRequested => {
                    let window_size = display.get_framebuffer_dimensions();
                    let aspect_ratio = window_size.1 as f32 / window_size.0 as f32;

                    let x_off = 0.1*t.sin();
                    t += 0.1;

                    let mut target = display.draw();
                    target.clear_color(0.0, 0.0, 0.0, 1.0);
                    target.draw(&vertex_buffer, &indices, &program, &uniform! { x_off: x_off, aspectRatio: aspect_ratio, resolution:  [window_size.0 as f32,window_size.1 as f32]},
                            &Default::default()).unwrap();
                    target.finish().unwrap();
                }
                    _ => (),
                },
                Event::AboutToWait => {
                    window.request_redraw();
                },
            _ => (),
        };
    });
}
