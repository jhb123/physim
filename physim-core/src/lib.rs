#![feature(test)]
#![feature(vec_into_raw_parts)]
#![feature(box_as_ptr)]

use libloading::Library;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde_json::{self, Value};
use std::{
    collections::HashMap,
    env,
    path::Path,
    sync::{
        atomic::{AtomicPtr, Ordering},
        mpsc::Receiver,
    },
};
use yansi::Paint;

#[derive(Debug)]
#[repr(C)]
pub enum ElementKind {
    Initialiser,
    Transform,
    Render,
}

// set by library authors, determined at compile time
#[derive(Debug)]
#[repr(C)]
pub struct ElementInfo {
    kind: ElementKind,
    name: String,
    plugin: String,
    version: String,
    license: String,
    author: String,
}

impl ElementInfo {
    pub fn new(
        kind: ElementKind,
        name: &str,
        plugin: &str,
        version: &str,
        license: &str,
        author: &str,
    ) -> Self {
        Self {
            kind,
            name: name.to_string(),
            plugin: plugin.to_string(),
            version: version.to_string(),
            license: license.to_string(),
            author: author.to_string(),
        }
    }
}
pub trait TransformElement {
    fn new(properties: HashMap<String, Value>) -> Self;
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32);
}

#[repr(C)]
pub struct TransformElementAPI {
    pub init: unsafe extern "C" fn(*const u8, usize) -> *mut std::ffi::c_void,
    pub transform:
        unsafe extern "C" fn(*mut std::ffi::c_void, *const Entity, usize, *mut Entity, usize, f32),
    pub destroy: unsafe extern "C" fn(*mut std::ffi::c_void),
}

pub struct TransformElementHandler {
    api: &'static TransformElementAPI,
    instance: AtomicPtr<std::ffi::c_void>,
    _lib: Library,
}

impl TransformElementHandler {
    pub fn load(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let api_fn_name = format!("{name}_get_api");
            let properties = match serde_json::to_string(&properties) {
                Ok(s) => s,
                Err(_) => return Err("Invalid config. Must be JSON".into()),
            };
            let lib = libloading::Library::new(path)?;
            let get_api: libloading::Symbol<unsafe extern "C" fn() -> *const TransformElementAPI> =
                lib.get(api_fn_name.as_bytes())?;
            let api = get_api();
            let (c, u, _l) = properties.into_raw_parts();
            let instance = ((*api).init)(c, u);
            if instance.is_null() {
                return Err("Could not initialise element".into());
            }
            Ok(Self {
                api: &*api,
                instance: AtomicPtr::new(instance),
                _lib: lib,
            })
        }
    }

    pub fn transform(&self, state: &[Entity], new_state: &mut [Entity], dt: f32) {
        let state_len = state.len();
        let state = state.as_ptr();
        let new_state_len = state_len;
        let new_state_ptr = new_state.as_mut_ptr();
        unsafe {
            (self.api.transform)(
                self.instance.load(Ordering::Relaxed),
                state,
                state_len,
                new_state_ptr,
                new_state_len,
                dt,
            );
            // new_state = std::slice::from_raw_parts_mut(new_state_ptr, new_state_len) ;
        }
    }

    pub fn destroy(&self) {
        unsafe {
            (self.api.destroy)(self.instance.load(Ordering::SeqCst));
        }
    }
}

impl Drop for TransformElementHandler {
    fn drop(&mut self) {
        self.destroy();
    }
}
pub trait RenderElementCreator {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn RenderElement>;
}

pub trait RenderElement {
    fn render(&mut self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>);
}
pub struct RenderElementHandler {
    instance: Box<dyn RenderElement>,
}

impl RenderElementHandler {
    pub fn load(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<RenderElementHandler, Box<dyn std::error::Error>> {
        unsafe {
            let fn_name = format!("{name}_create_element");
            let lib = libloading::Library::new(path)?;
            type GetNewFnType = unsafe extern "Rust" fn(
                properties: HashMap<String, Value>,
            ) -> Box<dyn RenderElement>;
            let get_new_fn: libloading::Symbol<GetNewFnType> = lib.get(fn_name.as_bytes())?;
            let ins = get_new_fn(properties);
            Ok(RenderElementHandler { instance: ins })
        }
    }

    pub fn render(&mut self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>) {
        self.instance.render(config, state_recv);
    }
}

pub trait InitialStateElementCreator {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement>;
}

pub trait InitialStateElement {
    fn initialise(&self) -> Vec<Entity>;
}
pub struct InitialStateElementHandler {
    instance: Box<dyn InitialStateElement>,
}

impl InitialStateElementHandler {
    pub fn load(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<InitialStateElementHandler, Box<dyn std::error::Error>> {
        unsafe {
            let fn_name = format!("{name}_create_element");
            let lib = libloading::Library::new(path)?;
            type GetNewFnType = unsafe extern "Rust" fn(
                properties: HashMap<String, Value>,
            )
                -> Box<dyn InitialStateElement>;
            let get_new_fn: libloading::Symbol<GetNewFnType> = lib.get(fn_name.as_bytes())?;
            let ins = get_new_fn(properties);
            Ok(InitialStateElementHandler { instance: ins })
        }
    }

    pub fn initialise(&mut self) -> Vec<Entity> {
        self.instance.initialise()
    }
}

#[repr(C)]
pub struct UniverseConfiguration {
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
    // edge_mode: UniverseEdge,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
pub struct Entity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub radius: f32,
    pub mass: f32,
}

impl Entity {
    pub fn new(x: f32, y: f32, z: f32, mass: f32) -> Self {
        Self {
            x,
            y,
            z,
            mass,
            radius: mass.powf(0.33333),
            ..Default::default()
        }
    }
    pub fn new2(x: f32, y: f32, z: f32, mass: f32, radius: f32) -> Self {
        Self {
            x,
            y,
            z,
            mass,
            radius,
            ..Default::default()
        }
    }

    pub fn random(rng: &mut ChaCha8Rng) -> Self {
        let x: f32 = rng.random_range(-1.0..1.0);
        let y: f32 = rng.random_range(-1.0..1.0);
        let z: f32 = rng.random_range(0.0..1.0);

        // let n = (x.powi(2) + y.powi(2)).powf(0.5);
        let vx: f32 = -(y) * 300.0;
        let vy: f32 = x * 300.0;

        // let vy: f32 = 0.05*(x)/n;

        Self {
            x,
            y,
            z,
            vx,
            vy,
            vz: 0.0,
            mass: 0.005,
            radius: 0.02,
        }
    }

    pub fn jitter_position(&self) -> Self {
        let mut rng = rand::rng();
        Self {
            x: self.x + rng.random_range(-0.01..0.01),
            y: self.y + rng.random_range(-0.01..0.01),
            z: self.z + rng.random_range(-0.01..0.01),
            radius: self.radius,
            ..Default::default()
        }
    }

    pub fn jitter_position_inplace(&mut self) {
        let mut rng = rand::rng();
        self.x += rng.random_range(-0.01..0.01);
        self.y += rng.random_range(-0.01..0.01);
        self.z += rng.random_range(-0.01..0.01);
    }
}

#[macro_export]
macro_rules! register_plugin {
    ( $( $x:expr ),* ) => {

        #[unsafe(no_mangle)]
        fn register_plugin() -> std::ffi::CString{
            let mut elements = Vec::new();
            $(
                elements.push($x);
            )*
            CString::new(elements.join(",")).unwrap_or_default()
        }
    };
}

// determined at run time
#[derive(Debug)]
pub struct RegisteredElement {
    element_info: ElementInfo,
    lib_path: String,
}

impl RegisteredElement {
    fn new(element_info: ElementInfo, lib_path: &str) -> Self {
        RegisteredElement {
            element_info,
            lib_path: lib_path.to_string(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.element_info.name
    }

    pub fn print_element_info_brief(&self) {
        match self.element_info.kind {
            ElementKind::Initialiser => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_cyan(),
                "initialiser".cyan().dim()
            ),
            ElementKind::Transform => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_green(),
                "transform".green().dim()
            ),
            ElementKind::Render => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_yellow(),
                "renderer".yellow().dim()
            ),
        }
    }

    pub fn print_element_info_verbose(&self) {
        println!(
            "{}: {}",
            "About".bold().bright().underline().green().linger(),
            self.element_info.name.resetting()
        );
        println!(
            "Loaded from the {} plugin located in {}",
            self.element_info.plugin.green(),
            self.lib_path.green()
        );
        println!();
        println!(
            "{:>10} - {:#?}",
            "Kind".bold(),
            self.element_info.kind.green()
        );
        println!(
            "{:>10} - {}",
            "Authors".bold(),
            self.element_info.author.green()
        );
        println!(
            "{:>10} - {}",
            "License".bold(),
            self.element_info.license.green()
        );
        println!(
            "{:>10} - {}",
            "Version".bold(),
            self.element_info.version.green()
        );
        println!();
    }
}

pub fn get_plugin_dir() -> String {
    env::var("PHYSIM_PLUGIN_DIR").unwrap_or("./".to_string())
}

pub fn discover() -> Vec<RegisteredElement> {
    let mut elements = Vec::new();
    let plugin_dir = get_plugin_dir();
    let plugin_dir = Path::new(&plugin_dir);
    if !plugin_dir.is_dir() {
        return Vec::new();
    }
    for entry in plugin_dir
        .read_dir()
        .expect("read_dir call failed")
        .flatten()
    {
        if let Some(ex) = entry.path().extension().and_then(|x| x.to_str()) {
            if ["dylib", "so", "dll"].contains(&ex) {
                log::info!("Scanning {:?}", entry);
                unsafe {
                    let lib_path = entry.path().to_str().expect("msg").to_string();
                    if let Ok(lib) = libloading::Library::new(&lib_path) {
                        if let Ok(register_plugin) = lib.get::<libloading::Symbol<
                            unsafe extern "C" fn() -> std::ffi::CString,
                        >>(
                            b"register_plugin"
                        ) {
                            let els = register_plugin().into_string().unwrap();
                            for el in els.split(",") {
                                let register_element =
                                        lib.get::<libloading::Symbol<
                                            unsafe extern "C" fn() -> ElementInfo,
                                        >>(
                                            format!("{el}_register").as_bytes()
                                        )
                                        .unwrap();
                                let element_info = register_element();

                                elements.push(RegisteredElement::new(element_info, &lib_path));
                            }
                        }
                    };
                }
            }
        }
    }
    elements
}
