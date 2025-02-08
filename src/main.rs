#[macro_use]
extern crate glium;

use std::cmp::max;

use glium::{
    winit::{
        event::{Event, WindowEvent},
        event_loop::EventLoop,
    },
    Surface,
};
use log::info;
use rand::Rng;

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

    fn random() -> Self {
        let mut rng = rand::rng();
        Self::new(
            [rng.random_range(-2.0..2.0), rng.random_range(-1.0..1.0)],
            rng.random_range(0.01..0.02),
        )
    }
}

enum UniverseEdge {
    Infinite,
    WrapAround,
}
struct UniverseConfiguration {
    size: [f32; 2],
    edge_mode: UniverseEdge,
}

/*
Coordinate systems
------------------
    Universe -> Physical units (from UOM)
    universe aspect ratio -> universe.x/universe.y
    Window size -> pixels
    Window aspect ratio -> window.x/window.y
    OpenGL Vertexes -> [-1, 1]

    Aim: remove all the calculations which involve the window size etc.
    https://glium.github.io/glium/book/tuto-10-perspective.html
*/

fn main() {
    env_logger::init();
    let event_loop = EventLoop::builder().build().expect("event loop building");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("PhySim Renderer")
        .build(&event_loop);

    let mut circles = Vec::with_capacity(1000);

    for _ in 0..1_000 {
        circles.push(Circle::random());
    }
    let verticies: Vec<Vertex> = circles.iter().flat_map(|s| s.verticies).collect();

    let vertex_buffer = glium::VertexBuffer::new(&display, &verticies).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 330

        in vec2 position;
        uniform float x_off;
        uniform mat4 matrix;       // new
        uniform mat4 perspective;       // new
        out vec4 vertexCoord;

        void main() {
            vec2 pos = position;
            pos.x += x_off;
            gl_Position = perspective*matrix*vec4(pos.x, pos.y , 0.01, 0.5);
            vertexCoord = gl_Position;
        }
    "#;

    let geometry_shader_src = r#"
        #version 330

        layout (triangles) in;
        layout(triangle_strip, max_vertices = 3) out; 
        uniform mat4 perspective;       // new
        out float radius;
        out vec4 centre;
        out vec4 fragCoord;

        void main() {   

            centre =  (gl_in[0].gl_Position + gl_in[1].gl_Position + gl_in[2].gl_Position)/3.0 ; 
            radius = (gl_in[0].gl_Position.y - centre.y)/2;

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
    "#;

    let fragment_shader_src = r#"
        #version 330

        // in vec2 fragCoord;
        in vec4 centre;
        in vec4 fragCoord;
        in float radius;
        out vec4 FragColor;
        uniform vec2 resolution;

        void main() {
            vec4 f = fragCoord;
            vec4 c = centre;
            f.x *= resolution[0]/resolution[1];
            c.x *= resolution[0]/resolution[1];


            if ( distance(f.xy,c.xy) > radius ){
                discard;
            } else {
                FragColor = vec4(1.0,1.0,1.0,1.0);
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

    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut zoom = 10.0;
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

                    let mut target = display.draw();

                    let perspective = {
                        let (width, height) = target.get_dimensions();
                        let aspect_ratio = height as f32 / width as f32;

                        let fov: f32 = 3.141592 / 3.0;
                        let zfar = 1024.0;
                        let znear = 0.1;
                        let f = 1.0 / (fov / 2.0).tan();

                        [
                            [f *   aspect_ratio   ,    0.0,              0.0              ,   0.0],
                            [         0.0         ,     f ,              0.0              ,   0.0],
                            [         0.0         ,    0.0,  (zfar+znear)/(zfar-znear)    ,   1.0],
                            [         0.0         ,    0.0, -(2.0*zfar*znear)/(zfar-znear),   0.0],
                        ]
                    };

                    let matrix = [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, zoom, 1.0f32] // . . zoom .
                    ];

                    target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
                    target.draw(&vertex_buffer, &indices, &program, &uniform! { matrix: matrix, perspective: perspective, resolution: [window_size.0 as f32,window_size.1 as f32]},
                            &params).unwrap();
                    target.finish().unwrap();
                },
                WindowEvent::KeyboardInput {
                    device_id, event: kin, is_synthetic: _
                } => {
                    if let glium::winit::event::ElementState::Released = kin.state {
                        if let glium::winit::keyboard::PhysicalKey::Code(key_code) = kin.physical_key {
                            match key_code {
                                glium::winit::keyboard::KeyCode::BracketLeft => zoom = (1.0 as f32).max(zoom/2.0),
                                glium::winit::keyboard::KeyCode::BracketRight => zoom = (10000 as f32).min(zoom*2.0),
                                _ => todo!(),
                            }
                        }
                    }
                },
                    _ => (),
                },
                Event::AboutToWait => {
                    window.request_redraw();
                },
            _ => (),
        };
    });
}
