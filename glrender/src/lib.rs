#![feature(str_from_raw_parts)]
use std::io::Write;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{collections::HashMap, f32::consts::PI, sync::mpsc::Receiver};

use glium::texture::RawImage2d;
use glium::winit::event::KeyEvent;
use glium::winit::keyboard::{KeyCode, PhysicalKey};
use glium::winit::window::Window;
use glium::{
    Surface, implement_vertex, uniform,
    winit::{
        event::{Event, WindowEvent},
        event_loop::EventLoop,
    },
};
use physim_attribute::render_element;
use physim_core::log::{debug, error, warn};
use physim_core::messages::{Message, MessageClient, MessagePriority};
use physim_core::plugin::render::RenderElement;
use physim_core::plugin::{Element, ElementCreator};
use physim_core::{Entity, msg, post_bus_msg, register_plugin};
use serde_json::Value;

register_plugin!("glrender,stdout");

const SHADER_DESC: &str =
    "yellowblue, velocity, rgb-velocity, smoke, twinkle, id, orange-blue, hot";

const MAX_BUFFER_SIZE: usize = 10_000_000;

struct RenderConfiguration {
    size_x: f64,
    size_y: f64,
}

const RENDER_CONFIG: RenderConfiguration = RenderConfiguration {
    size_x: 2.0,
    size_y: 1.0,
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    velocity: [f32; 3],
    id: u32,
}

#[derive(Default)]
enum RenderPipelineShader {
    #[default]
    YellowBlue,
    Velocity,
    RgbVelocity,
    OrangeBlue,
    Hot,
    Smoke,
    Twinkle,
    Id,
}

impl FromStr for RenderPipelineShader {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "yellowblue" | "yellow-blue" => Ok(RenderPipelineShader::YellowBlue),
            "velocity" => Ok(RenderPipelineShader::Velocity),
            "orange-blue" => Ok(RenderPipelineShader::OrangeBlue),
            "hot" => Ok(RenderPipelineShader::Hot),
            "rgbvelocity" | "rgb-velocity" => Ok(RenderPipelineShader::RgbVelocity),
            "smoke" => Ok(RenderPipelineShader::Smoke),
            "twinkle" => Ok(RenderPipelineShader::Twinkle),
            "id" => Ok(RenderPipelineShader::Id),
            _ => Err(()),
        }
    }
}

trait Renderable {
    fn vertices(&self) -> Vec<Vertex>;
}

impl Renderable for Entity {
    fn vertices(&self) -> Vec<Vertex> {
        vec![
            Vertex {
                position: [
                    self.x as f32,
                    self.y as f32 + (self.radius * 2.0) as f32,
                    self.z as f32,
                ],
                velocity: [self.vx as f32, self.vy as f32, self.vx as f32],
                id: self.id as u32,
            },
            Vertex {
                position: [
                    (self.x - self.radius * 2.0 * f64::sqrt(3.0) * 0.5) as f32,
                    (self.y - 0.5 * self.radius * 2.0) as f32,
                    self.z as f32,
                ],
                velocity: [self.vx as f32, self.vy as f32, self.vx as f32],
                id: self.id as u32,
            },
            Vertex {
                position: [
                    (self.x + self.radius * 2.0 * f64::sqrt(3.0) * 0.5) as f32,
                    (self.y - 0.5 * self.radius * 2.0) as f32,
                    self.z as f32,
                ],
                velocity: [self.vx as f32, self.vy as f32, self.vx as f32],
                id: self.id as u32,
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
                include_str!("velocity/blue-green.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
            Self::RgbVelocity => (
                include_str!("velocity/rgb-velocity.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
            Self::Smoke => (
                include_str!("velocity/smoke.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
            Self::Twinkle => (
                include_str!("velocity/twinkle.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
            Self::Id => (
                include_str!("id/rgb.vert"),
                include_str!("id/shader.geom"),
                include_str!("id/shader.frag"),
            ),
            Self::OrangeBlue => (
                include_str!("velocity/orange-blue.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
            Self::Hot => (
                include_str!("velocity/hot.vert"),
                include_str!("velocity/shader.geom"),
                include_str!("velocity/shader.frag"),
            ),
        }
    }
}

implement_vertex!(Vertex, position, velocity, id);

#[render_element(name = "glrender", blurb = "Render simulation to a window")]
pub struct GLRenderElement {
    inner: Mutex<InnerRenderElement>,
}

struct InnerRenderElement {
    resolution: (u64, u64),
    zoom: f32,
    shader: RenderPipelineShader,
    running: bool,
}

impl GLRenderElement {
    fn keycode_msg(&self, key: &KeyEvent) -> Option<Message> {
        let key = key.text.clone()?;
        Some(msg!(
            self,
            "keyboard.press",
            key.to_string(),
            MessagePriority::Normal
        ))
    }
}

impl ElementCreator for GLRenderElement {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
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

        let shader = RenderPipelineShader::from_str(shader).unwrap_or_default();

        Box::new(GLRenderElement {
            inner: Mutex::new(InnerRenderElement {
                resolution,
                zoom,
                shader,
                running: true,
            }),
        })
    }
}

impl RenderElement for GLRenderElement {
    fn render(&self, state_recv: Receiver<Vec<Entity>>) {
        let element = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let event_loop = EventLoop::builder().build().expect("event loop building");
        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("PhySim Renderer")
            .with_inner_size(element.resolution.0 as u32, element.resolution.1 as u32)
            .build(&event_loop);

        let state: Vec<Entity> = match state_recv.recv() {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut vertices: Vec<Vertex> = state.iter().flat_map(|s| s.vertices()).collect();

        let vertex_buffer = match glium::VertexBuffer::empty_dynamic(&display, MAX_BUFFER_SIZE) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to create vertex buffer: {}", e);
                std::process::exit(1)
            }
        };
        let max_rendered = std::cmp::min(MAX_BUFFER_SIZE, vertices.len());
        if max_rendered > 0 {
            vertex_buffer
                .slice(0..max_rendered)
                .expect("Failed to get slice for vertex buffer")
                .write(&vertices[0..max_rendered]);
        }

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let (vertex_shader_src, geometry_shader_src, fragment_shader_src) =
            element.shader.get_shader();

        let program = match glium::Program::from_source(
            &display,
            vertex_shader_src,
            fragment_shader_src,
            Some(geometry_shader_src),
        ) {
            Ok(program) => program,
            Err(e) => {
                eprintln!("Failed to created shader program: {}", e);
                std::process::exit(1)
            }
        };

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

        let mut zoom = (RENDER_CONFIG.size_x.max(RENDER_CONFIG.size_y) as f32) * element.zoom;
        let mut pos_x = 0.0;
        let mut pos_y = 0.0;
        let mut frame_num = 0;

        drop(element);
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
                    let n = RENDER_CONFIG.size_x.max(RENDER_CONFIG.size_y) as f32;
                    let matrix = [
                        [1.0/n, 0.0, 0.0, 0.0],
                        [0.0, 1.0/n, 0.0, 0.0],
                        [0.0, 0.0, 1.0/n, 0.0],
                        [0.0, 0.0, zoom, 1.0f32] // move x, move y, zoom, .
                    ];

                    let uniforms = uniform! { matrix: matrix, perspective: perspective, resolution: [window_size.0 as f32,window_size.1 as f32], xy_off: [pos_x,pos_y], frame_num: frame_num};

                    target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
                    if let Err(e) = target.draw(&vertex_buffer, indices, &program, &uniforms,
                            &params) {
                                eprintln!("Failed to render {}",e);
                                std::process::exit(1);
                            };
                    target.finish().expect("Unlikely. This can fail on mobile");
                    frame_num +=1;
                },
                WindowEvent::KeyboardInput {
                    device_id: _, event: kin, is_synthetic: _
                } => {
                    if let Some(message) = self.keycode_msg(&kin) {
                        post_bus_msg!(message);
                    }
                    if let glium::winit::event::ElementState::Released = kin.state {
                        if let PhysicalKey::Code(key_code) = kin.physical_key {
                            match key_code {
                                KeyCode::BracketLeft => zoom = 0.01_f32.max(zoom/2.0),
                                KeyCode::BracketRight => zoom = 4_f32.min(zoom*2.0),
                                _ => {},
                            }
                        }
                    }
                    if let glium::winit::event::ElementState::Pressed  = kin.state {
                        if let PhysicalKey::Code(key_code) = kin.physical_key {
                            match key_code {
                                KeyCode::KeyS => pos_y = 1.0_f32.min(pos_y + 0.1*zoom),
                                KeyCode::KeyW => pos_y = (-1.0_f32).max(pos_y - 0.1*zoom),
                                KeyCode::KeyA => pos_x = 1.0_f32.min(pos_x + 0.1*zoom),
                                KeyCode::KeyD => pos_x = (-1.0_f32).max(pos_x - 0.1*zoom),
                                KeyCode::KeyP | KeyCode::Space  => {
                                    let pause = msg!(self,"pipeline","pause_toggle",MessagePriority::High);
                                    post_bus_msg!(pause);
                                },
                                KeyCode::KeyQ => {
                                    let pause = msg!(self,"pipeline","quit",MessagePriority::High);
                                    post_bus_msg!(pause);
                                },
                            _ => {},
                            }
                        }
                    }
                },
                    _ => (),
                },
                Event::AboutToWait => {
                    if !self.inner.lock().unwrap_or_else(|e| e.into_inner()).running {
                        window_target.exit()
                    }
                    debug!("Waiting for next state update");
                    vertices.clear();
                    if let Ok(state) = state_recv.recv() {
                        vertices.extend(state.iter().flat_map(|s| s.vertices()));
                        let max_rendered = std::cmp::min(vertex_buffer.len() ,vertices.len());
                        vertex_buffer.invalidate();
                        if max_rendered > 0 {
                        vertex_buffer
                            .slice(0..max_rendered)
                            .expect("Failed to get slice for vertex buffer")
                            .write(&vertices[0..max_rendered]);
                        }
                        window.request_redraw();
                    } else {
                        window_target.exit();
                    }
                },
            _ => (),
        };
    });
    }
}

impl Element for GLRenderElement {
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
            ("shader".to_string(), SHADER_DESC.to_string()),
        ]))
    }
}

impl MessageClient for GLRenderElement {
    fn recv_message(&self, message: &physim_core::messages::Message) {
        if &message.topic == "pipeline" && &message.message == "finished" {
            self.inner.lock().unwrap_or_else(|e| e.into_inner()).running = false
        }
    }
}

#[render_element(
    name = "stdout",
    blurb = "Render simulation to stdout as 8bit RGBA pixels for further processing by video software"
)]
pub struct StdOutRender {
    inner: Mutex<InnerRenderElement>,
    buffer_size: usize,
    capture_frame: Option<usize>,
    counter: AtomicUsize,
}

struct FrameBuffer {
    buffer: Vec<u8>,
    buffered_frames: usize,
    stdout: std::io::Stdout,
    max_frames: usize,
}

impl FrameBuffer {
    fn new(frames: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(4 * 3840 * 2160 * frames),
            buffered_frames: 0,
            max_frames: frames,
            stdout: std::io::stdout(),
        }
    }

    fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.buffered_frames += 1;
        if self.buffered_frames == self.max_frames {
            self.flush();
        }
    }

    fn flush(&mut self) {
        let mut handle = std::io::BufWriter::new(self.stdout.lock());
        if let Err(e) = handle.write_all(&self.buffer) {
            warn!("Failed to write pixels to stdout: {}", e);
        }
        handle.flush().expect("Failed to flush stdout");
        self.buffer.clear();
        self.buffered_frames = 0;
    }
}

impl ElementCreator for StdOutRender {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
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

        let buffer_size = properties
            .get("buffer")
            .and_then(|v| v.as_u64().map(|x| x as usize))
            .unwrap_or(30);

        let capture_frame = properties
            .get("frame")
            .and_then(|v| v.as_u64().map(|x| x as usize));

        let shader = RenderPipelineShader::from_str(shader).unwrap_or_default();

        Box::new(StdOutRender {
            inner: Mutex::new(InnerRenderElement {
                resolution,
                zoom,
                shader,
                running: true,
            }),
            buffer_size,
            capture_frame,
            counter: AtomicUsize::new(0),
        })
    }
}

impl RenderElement for StdOutRender {
    fn render(&self, state_recv: Receiver<Vec<Entity>>) {
        match std::panic::catch_unwind(|| {
            let mut pixel_buffer = FrameBuffer::new(self.buffer_size);

            let element = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let mut vertices: Vec<Vertex> = match state_recv.recv() {
                Ok(state) => state.iter().flat_map(|s| s.vertices()).collect(),
                Err(e) => {
                    error!("Failed to receive state {}", e);
                    return;
                }
            };

            let event_loop = EventLoop::builder().build().expect("event loop building");
            let (_, display) = glium::backend::glutin::SimpleWindowBuilder::new()
                .set_window_builder(Window::default_attributes().with_visible(false))
                .with_title("PhySim Renderer")
                .with_inner_size(element.resolution.0 as u32, element.resolution.1 as u32)
                .build(&event_loop);

            let vertex_buffer = match glium::VertexBuffer::empty_dynamic(&display, MAX_BUFFER_SIZE)
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to create vertex buffer: {}", e);
                    std::process::exit(1)
                }
            };
            let max_rendered = std::cmp::min(MAX_BUFFER_SIZE, vertices.len());
            if max_rendered > 0 {
                vertex_buffer
                    .slice(0..max_rendered)
                    .expect("Failed to get slice for vertex buffer")
                    .write(&vertices[0..max_rendered]);
            }

            let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

            let (vertex_shader_src, geometry_shader_src, fragment_shader_src) =
                element.shader.get_shader();

            let program = match glium::Program::from_source(
                &display,
                vertex_shader_src,
                fragment_shader_src,
                Some(geometry_shader_src),
            ) {
                Ok(program) => program,
                Err(e) => {
                    eprintln!("Failed to created shader program: {}", e);
                    std::process::exit(1)
                }
            };

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

            let zoom = (RENDER_CONFIG.size_x.max(RENDER_CONFIG.size_y) as f32) * element.zoom;
            let pos_x: f32 = 0.0;
            let pos_y: f32 = 0.0;

            drop(element);

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
            let n = RENDER_CONFIG.size_x.max(RENDER_CONFIG.size_y) as f32;
            let matrix = [
                [1.0 / n, 0.0, 0.0, 0.0],
                [0.0, 1.0 / n, 0.0, 0.0],
                [0.0, 0.0, 1.0 / n, 0.0],
                [0.0, 0.0, zoom, 1.0f32], // move x, move y, zoom, .
            ];
            display.resize(window_size);
            target.finish().expect("Unlikely. This can fail on mobile");

            loop {
                let current_frame = self.counter.fetch_add(1, Ordering::Relaxed);
                vertices.clear();
                match state_recv.recv() {
                    Ok(state) => {
                        vertices.extend(state.iter().flat_map(|s| s.vertices()));
                    }
                    Err(_) => {
                        pixel_buffer.flush();
                        break;
                    }
                }
                if self.capture_frame.is_some()
                    && !self.capture_frame.is_some_and(|f| f == current_frame)
                {
                    continue;
                }

                let max_rendered = std::cmp::min(vertex_buffer.len(), vertices.len());
                vertex_buffer.invalidate();
                if max_rendered > 0 {
                    vertex_buffer
                        .slice(0..max_rendered)
                        .expect("Failed to get slice for vertex buffer")
                        .write(&vertices[0..max_rendered]);
                }

                target = display.draw();
                target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
                if let Err(e) = target.draw(
                    &vertex_buffer,
                    indices,
                    &program,
                    &uniform! {
                        matrix: matrix,
                        perspective: perspective,
                        resolution: [window_size.0 as f32,window_size.1 as f32], xy_off: [pos_x,pos_y]
                    },
                    &params,
                ) {
                    eprintln!("Failed to render {}", e);
                    std::process::exit(1);
                };

                target
                    .finish()
                    .expect("Unlikely, this can happen on mobile");
                match display.read_front_buffer::<RawImage2d<u8>>() {
                    Ok(image) => {
                        pixel_buffer.push(&image.data);
                    }
                    Err(e) => {
                        error!("Failed to collect pixels: {e}");
                        std::process::exit(1)
                    }
                };
                if self.capture_frame.is_some()
                    && self.capture_frame.is_some_and(|f| f == current_frame)
                {
                    pixel_buffer.flush();
                    break;
                }
            }
        }) {
            Ok(_) => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

impl Element for StdOutRender {
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
            ("shader".to_string(), SHADER_DESC.to_string()),
            (
                "buffer_size".to_string(),
                "Number of frames to buffer before writing".to_string(),
            ),
            (
                "frame".to_string(),
                "If specified, only one frame will be generated at the timestep specified by frame"
                    .to_string(),
            ),
        ]))
    }
}

impl MessageClient for StdOutRender {}
