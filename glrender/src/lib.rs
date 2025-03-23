#![feature(str_from_raw_parts)]
use std::ffi::CString;
use std::{collections::HashMap, f32::consts::PI, sync::mpsc::Receiver};

use glium::{
    Surface, implement_vertex, uniform,
    winit::{
        event::{Event, WindowEvent},
        event_loop::EventLoop,
    },
};
use physim_attribute::render_element;
use physim_core::{
    ElementInfo, ElementKind, Entity, RenderElement, RenderElementCreator, UniverseConfiguration,
    register_plugin,
};
use serde_json::Value;

register_plugin!("glrender");

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
}

trait Renderable {
    fn verticies(&self) -> Vec<Vertex>;
}

impl Renderable for Entity {
    fn verticies(&self) -> Vec<Vertex> {
        vec![
            Vertex::new(self.x, self.y + self.radius, self.z),
            Vertex::new(
                self.x - self.radius * f32::sqrt(3.0) * 0.5,
                self.y - 0.5 * self.radius,
                self.z,
            ),
            Vertex::new(
                self.x + self.radius * f32::sqrt(3.0) * 0.5,
                self.y - 0.5 * self.radius,
                self.z,
            ),
        ]
    }
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
        }
    }
}

implement_vertex!(Vertex, position);

#[render_element("glrender")]
pub struct GLRenderElement {}

impl RenderElementCreator for GLRenderElement {
    fn create_element(_properties: HashMap<String, Value>) -> Box<dyn RenderElement> {
        Box::new(GLRenderElement {})
    }
}

impl RenderElement for GLRenderElement {
    fn render(&mut self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>) {
        // Receiver<T>

        // let mut verticies: Vec<Vertex> =  Vec::with_capacity(2000000);// state_recv.recv().unwrap().iter().flat_map(|s| s.verticies()).collect();
        let mut verticies: Vec<Vertex> = state_recv
            .recv()
            .unwrap()
            .iter()
            .flat_map(|s| s.verticies())
            .collect();

        let event_loop = EventLoop::builder().build().expect("event loop building");
        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("PhySim Renderer")
            .build(&event_loop);

        let vertex_buffer = glium::VertexBuffer::empty_dynamic(&display, 10_000_000).unwrap();
        vertex_buffer
            .slice(0..verticies.len())
            .unwrap()
            .write(&verticies);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let vertex_shader_src = include_str!("shader.vert");
        let geometry_shader_src = include_str!("shader.geom");
        let fragment_shader_src = include_str!("shader.frag");

        let program = glium::Program::from_source(
            &display,
            vertex_shader_src,
            fragment_shader_src,
            Some(geometry_shader_src),
        )
        .unwrap();

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise, // Do I actually need this?
            ..Default::default()
        };

        let mut zoom = config.size_x.max(config.size_y);
        let mut pos_x = 0.0;
        let mut pos_y = 0.0;

        // thread::spawn(move || {
        // *verticies.lock().unwrap() = new_state;
        // });

        // this avoids a lot of boiler plate.
        #[allow(deprecated)]
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

                        let fov: f32 = PI / 3.0;
                        let zfar = 8.0;
                        let znear = 0.01;
                        let f = 1.0 / (fov / 3.0).tan();

                        [
                            [f *   aspect_ratio   ,    0.0,              0.0              ,   0.0],
                            [         0.0         ,     f ,              0.0              ,   0.0],
                            [         0.0         ,    0.0,  (zfar+znear)/(zfar-znear)    ,   1.0],
                            [         0.0         ,    0.0, -(2.0*zfar*znear)/(zfar-znear),   0.0],
                        ]
                    };
                    let n = config.size_x.max(config.size_y);
                    let matrix = [
                        [1.0/n, 0.0, 0.0, 0.0],
                        [0.0, 1.0/n, 0.0, 0.0],
                        [0.0, 0.0, 1.0/n, 0.0],
                        [0.0, 0.0, zoom, 1.0f32] // move x, move y, zoom, .
                    ];

                    target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
                    target.draw(&vertex_buffer, indices, &program, &uniform! { matrix: matrix, perspective: perspective, resolution: [window_size.0 as f32,window_size.1 as f32], xy_off: [pos_x,pos_y]},
                            &params).unwrap();
                    target.finish().unwrap();
                },
                WindowEvent::KeyboardInput {
                    device_id: _, event: kin, is_synthetic: _
                } => {
                    if let glium::winit::event::ElementState::Released = kin.state {
                        if let glium::winit::keyboard::PhysicalKey::Code(key_code) = kin.physical_key {
                            match key_code {
                                glium::winit::keyboard::KeyCode::BracketLeft => zoom = 0.01_f32.max(zoom/2.0),
                                glium::winit::keyboard::KeyCode::BracketRight => zoom = 4_f32.min(zoom*2.0),
                                _ => {},
                            }
                        }
                    }
                    if let glium::winit::event::ElementState::Pressed  = kin.state {
                        if let glium::winit::keyboard::PhysicalKey::Code(key_code) = kin.physical_key {
                            match key_code {
                                glium::winit::keyboard::KeyCode::KeyS => pos_y = 1.0_f32.min(pos_y + 0.1*zoom),
                                glium::winit::keyboard::KeyCode::KeyW => pos_y = (-1.0_f32).max(pos_y - 0.1*zoom),
                                glium::winit::keyboard::KeyCode::KeyA => pos_x = 1.0_f32.min(pos_x + 0.1*zoom),
                                glium::winit::keyboard::KeyCode::KeyD => pos_x = (-1.0_f32).max(pos_x - 0.1*zoom),
                            _ => {},
                            }
                        }
                    }
                },
                    _ => (),
                },
                Event::AboutToWait => {
                    verticies.clear();
                    verticies.extend(state_recv.recv().unwrap().iter().flat_map(|s| s.verticies()));
                    vertex_buffer.invalidate();
                    vertex_buffer.slice(0..verticies.len()).unwrap().write(&verticies);
                    window.request_redraw();
                },
            _ => (),
        };
    });
    }
}
