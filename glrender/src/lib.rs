#![feature(str_from_raw_parts)]
use std::ffi::CString;
use std::io::Write;
use std::{collections::HashMap, f32::consts::PI, sync::mpsc::Receiver};

use glium::{
    Surface, implement_vertex, uniform,
    winit::{
        event::{Event, WindowEvent},
        event_loop::EventLoop,
    },
};
use physim_attribute::render_element;
use physim_core::plugin::render::{RenderElement, RenderElementCreator};
use physim_core::{Entity, UniverseConfiguration, register_plugin};
use serde_json::Value;

register_plugin!("glrender,stdout");

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    velocity: [f32; 3],
}

enum RenderPipelineShader {
    YellowBlue,
    Velocity,
}

trait Renderable {
    fn verticies(&self) -> Vec<Vertex>;
}

impl Renderable for Entity {
    fn verticies(&self) -> Vec<Vertex> {
        vec![
            Vertex {
                position: [self.state.x, self.state.y + self.radius, self.state.z],
                velocity: [self.state.vx, self.state.vy, self.state.vx],
            },
            Vertex {
                position: [
                    self.state.x - self.radius * f32::sqrt(3.0) * 0.5,
                    self.state.y - 0.5 * self.radius,
                    self.state.z,
                ],
                velocity: [self.state.vx, self.state.vy, self.state.vx],
            },
            Vertex {
                position: [
                    self.state.x + self.radius * f32::sqrt(3.0) * 0.5,
                    self.state.y - 0.5 * self.radius,
                    self.state.z,
                ],
                velocity: [self.state.vx, self.state.vy, self.state.vx],
            },
        ]
    }
}

impl RenderPipelineShader {
    fn get_shader(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::YellowBlue => (
                include_str!("yellowblue/shader.vert"),
                include_str!("yellowblue/shader.geom"),
                include_str!("yellowblue/shader.frag"),
            ),
            Self::Velocity => (
                include_str!("velocity/shader.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
        }
    }
}

impl Default for RenderPipelineShader {
    fn default() -> Self {
        Self::YellowBlue
    }
}

implement_vertex!(Vertex, position, velocity);

#[render_element(name = "glrender", blurb = "Render simulation to a window")]
pub struct GLRenderElement {
    resolution: (u64, u64),
    zoom: f32,
    shader: RenderPipelineShader,
}

impl RenderElementCreator for GLRenderElement {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn RenderElement> {
        let resolution = properties
            .get("resolution")
            .and_then(|theta| theta.as_str())
            .map(|s| match s {
                "1080p" => (1920, 1080),
                "720p" => (1280, 720),
                "4k" => (3840, 2160),
                _ => (1920, 1080),
            })
            .unwrap_or((1920, 1080));

        let zoom = properties
            .get("zoom")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let shader = properties
            .get("shader")
            .and_then(|s| s.as_str())
            .unwrap_or_default();

        let shader = match shader {
            "yellowblue" => RenderPipelineShader::YellowBlue,
            "velocity" => RenderPipelineShader::Velocity,
            _ => RenderPipelineShader::default(),
        };

        Box::new(GLRenderElement {
            resolution,
            zoom,
            shader,
        })
    }
}

impl RenderElement for GLRenderElement {
    fn render(&mut self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>) {
        let event_loop = EventLoop::builder().build().expect("event loop building");
        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("PhySim Renderer")
            .with_inner_size(self.resolution.0 as u32, self.resolution.1 as u32)
            .build(&event_loop);

        let state = state_recv.recv().unwrap();

        let mut verticies: Vec<Vertex> = state.iter().flat_map(|s| s.verticies()).collect();

        let vertex_buffer = glium::VertexBuffer::empty_dynamic(&display, 10_000_000).unwrap();
        vertex_buffer
            .slice(0..verticies.len())
            .unwrap()
            .write(&verticies);

        // vertex_buffer.write(verticies2);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let (vertex_shader_src, geometry_shader_src, fragment_shader_src) =
            self.shader.get_shader();

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

        let mut zoom = config.size_x.max(config.size_y) * self.zoom;
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

                    let uniforms = uniform! { matrix: matrix, perspective: perspective, resolution: [window_size.0 as f32,window_size.1 as f32], xy_off: [pos_x,pos_y]};

                    target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
                    target.draw(&vertex_buffer, indices, &program, &uniforms,
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

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(resolution) = new_props.get("resolution").and_then(|theta| theta.as_str()) {
            match resolution {
                "1080p" => self.resolution = (1920, 1080),
                "720p" => self.resolution = (1280, 720),
                "4k" => self.resolution = (3840, 2160),
                _ => self.resolution = (1920, 1080),
            }
        }
        if let Some(zoom) = new_props.get("zoom").and_then(|zoom| zoom.as_f64()) {
            self.zoom = zoom as f32
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "resolution" => Ok(serde_json::json!(self.resolution)), // serialise back to 1080p or something?
            "zoom" => Ok(serde_json::json!(self.zoom)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            (
                "resolution".to_string(),
                "Choices of 4k, 1080p and 720p".to_string(),
            ),
            (
                "zoom".to_string(),
                "Camera zoom (1.0 is default)".to_string(),
            ),
            ("shader".to_string(), "velocity, yellowblue".to_string()),
        ]))
    }
}

#[render_element(
    name = "stdout",
    blurb = "Render simulation to stdout for further processing by video software"
)]
pub struct StdOutRender {
    resolution: (u64, u64),
    zoom: f32,
    shader: RenderPipelineShader,
}

impl RenderElementCreator for StdOutRender {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn RenderElement> {
        let resolution = properties
            .get("resolution")
            .and_then(|theta| theta.as_str())
            .map(|s| match s {
                "1080p" => (1920, 1080),
                "720p" => (1280, 720),
                "4k" => (3840, 2160),
                _ => (1920, 1080),
            })
            .unwrap_or((1920, 1080));

        let zoom = properties
            .get("zoom")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let shader = properties
            .get("shader")
            .and_then(|s| s.as_str())
            .unwrap_or_default();

        let shader = match shader {
            "yellowblue" => RenderPipelineShader::YellowBlue,
            "velocity" => RenderPipelineShader::Velocity,
            _ => RenderPipelineShader::default(),
        };

        Box::new(StdOutRender {
            resolution,
            zoom,
            shader,
        })
    }
}

impl RenderElement for StdOutRender {
    fn render(&mut self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>) {
        let mut verticies: Vec<Vertex> = state_recv
            .recv()
            .unwrap()
            .iter()
            .flat_map(|s| s.verticies())
            .collect();

        let event_loop = EventLoop::builder().build().expect("event loop building");
        let (_, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("PhySim Renderer")
            .with_inner_size(self.resolution.0 as u32, self.resolution.1 as u32)
            .build(&event_loop);

        let vertex_buffer = glium::VertexBuffer::empty_dynamic(&display, 10_000_000).unwrap();
        vertex_buffer
            .slice(0..verticies.len())
            .unwrap()
            .write(&verticies);

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let (vertex_shader_src, geometry_shader_src, fragment_shader_src) =
            self.shader.get_shader();

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

        let zoom = config.size_x.max(config.size_y) * self.zoom;
        let pos_x: f32 = 0.0;
        let pos_y: f32 = 0.0;

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
                [f * aspect_ratio, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
            ]
        };
        let n = config.size_x.max(config.size_y);
        let matrix = [
            [1.0 / n, 0.0, 0.0, 0.0],
            [0.0, 1.0 / n, 0.0, 0.0],
            [0.0, 0.0, 1.0 / n, 0.0],
            [0.0, 0.0, zoom, 1.0f32], // move x, move y, zoom, .
        ];
        display.resize(window_size);
        target.finish().unwrap();

        let stdout = std::io::stdout();

        loop {
            verticies.clear();
            match state_recv.recv() {
                Ok(state) => {
                    verticies.extend(state.iter().flat_map(|s| s.verticies()));
                }
                Err(_) => {
                    break;
                }
            }
            vertex_buffer.invalidate();
            vertex_buffer
                .slice(0..verticies.len())
                .unwrap()
                .write(&verticies);

            target = display.draw();
            target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
            target.draw(&vertex_buffer, indices, &program, &uniform! { matrix: matrix, perspective: perspective, resolution: [window_size.0 as f32,window_size.1 as f32], xy_off: [pos_x,pos_y]},
                            &params).unwrap();
            target.finish().unwrap();
            #[allow(clippy::type_complexity)]
            let pixels: Result<Vec<Vec<(u8, u8, u8, u8)>>, _> = display.read_front_buffer();
            let pixels: Vec<u8> = pixels
                .unwrap()
                .iter()
                .rev()
                .flatten()
                .flat_map(|&(r, g, b, a)| vec![b, g, r, a])
                .collect();
            let mut handle = stdout.lock();
            handle
                .write_all(&pixels)
                .expect("Failed to write to stdout");
            handle.flush().expect("Failed to flush stdout");
        }
    }

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(resolution) = new_props.get("resolution").and_then(|theta| theta.as_str()) {
            match resolution {
                "1080p" => self.resolution = (1920, 1080),
                "720p" => self.resolution = (1280, 720),
                "4k" => self.resolution = (3840, 2160),
                _ => self.resolution = (1920, 1080),
            }
        }
        if let Some(zoom) = new_props.get("zoom").and_then(|zoom| zoom.as_f64()) {
            self.zoom = zoom as f32
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "resolution" => Ok(serde_json::json!(self.resolution)), // serialise back to 1080p or something?
            "zoom" => Ok(serde_json::json!(self.zoom)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            (
                "resolution".to_string(),
                "Choices of 4k, 1080p and 720p".to_string(),
            ),
            (
                "zoom".to_string(),
                "Camera zoom (1.0 is default)".to_string(),
            ),
            ("shader".to_string(), "velocity, yellowblue".to_string()),
        ]))
    }
}
