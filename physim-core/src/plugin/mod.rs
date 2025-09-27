use std::{
    collections::HashMap,
    env,
    error::Error,
    path::Path,
    sync::{Arc, Mutex},
};

use serde_json::Value;
use yansi::Paint;

use crate::{
    messages::{MessageBus, MessageClient},
    plugin::transform::TransformElementHandler,
};

pub mod generator;
pub mod integrator;
pub mod render;
pub mod transform;
pub mod transmute;

const PHYSIM_PLUGIN_LOADER_RUSTC_VERSION: &str = env!("ABI_INFO");

#[derive(Debug)]
#[repr(C)]
pub enum ElementKind {
    Initialiser,
    Transform,
    Render,
    Synth,
    Transmute,
    Integrator,
}

pub trait Element: MessageClient {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>>;
}

pub trait Loadable {
    type Item;
    fn load(
        path: &str,
        name: &str,
        properties: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let ins = unsafe { load_from_library::<Self::Item>(path, name, properties)? };
        Ok(Arc::new(Self::new(ins)))
    }

    fn new(instance: Self::Item) -> Self;
}

unsafe fn load_from_library<T>(
    path: &str,
    name: &str,
    properties: HashMap<String, Value>,
) -> Result<T, Box<dyn Error>> {
    let fn_name = format!("{name}_create_element");
    let lib = libloading::Library::new(path)?;
    type GetNewFnType<T> = unsafe extern "Rust" fn(HashMap<String, Value>) -> T;

    let get_new_fn: libloading::Symbol<GetNewFnType<T>> = lib.get(fn_name.as_bytes())?;
    Ok(get_new_fn(properties))
}

pub trait ElementCreator {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self>;
}

// set by library authors, determined at compile time
#[derive(Debug)]
#[repr(C)]
pub struct ElementMeta {
    kind: ElementKind,
    name: String,
    plugin: String,
    version: String,
    license: String,
    author: String,
    blurb: String,
    repo: String,
}

impl ElementMeta {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ElementKind,
        name: &str,
        plugin: &str,
        version: &str,
        license: &str,
        author: &str,
        blurb: &str,
        repo: &str,
    ) -> Self {
        Self {
            kind,
            name: name.to_string(),
            plugin: plugin.to_string(),
            version: version.to_string(),
            license: license.to_string(),
            author: author.to_string(),
            blurb: blurb.to_string(),
            repo: repo.to_string(),
        }
    }
}

/// Registers a plugin by generating a `register_plugin`, a
///  `set_callback_target` function and a `setup_logger` function.
///
/// # Usage
/// ```ignore
/// register_plugin!("my_plugin", "other_plugin");
/// ```
///
/// This macro is meant to be called once per crate in lib.rs to initialize the plugin interface.
#[macro_export]
macro_rules! register_plugin {
    ( $( $x:expr ),* ) => {

        pub const PLUGIN_ABI_INFO: &str = env!("ABI_INFO");

        #[unsafe(no_mangle)]
        pub extern "C" fn get_plugin_abi_info() -> std::ffi::CString {
            std::ffi::CString::new(PLUGIN_ABI_INFO).unwrap_or_default()
        }

        #[unsafe(no_mangle)]
        fn register_plugin() -> std::ffi::CString{
            let mut elements = Vec::new();
            $(
                elements.push($x);
            )*
            std::ffi::CString::new(elements.join(",")).unwrap_or_default()
        }

        static mut GLOBAL_BUS_TARGET: *mut std::ffi::c_void = std::ptr::null_mut();
        #[unsafe(no_mangle)]
        pub extern "C" fn set_callback_target(target: *mut std::ffi::c_void) {
            unsafe {
                assert!(!target.is_null());
                GLOBAL_BUS_TARGET = target;
            }
        }

        static LOGGER: $crate::once_cell::sync::OnceCell<Result<(), $crate::log::SetLoggerError>> = $crate::once_cell::sync::OnceCell::new();

        #[no_mangle]
        pub extern "Rust" fn setup_logger(
            logger: &'static dyn $crate::log::Log,
            level: $crate::log::LevelFilter,
        ) -> &Result<(), $crate::log::SetLoggerError> {
            LOGGER.get_or_init(|| {
                $crate::log::set_max_level(level);
                $crate::log::set_logger(logger)
            })
        }
    };
}

/// Sends a message to the global plugin bus target set by `set_callback_target`.
///
/// This macro wraps a call to the `callback` function using the globally stored
/// target pointer. It expects that `set_callback_target` has already been called
/// to initialize the global bus target. This can be done with the register_plugin
/// macro.
///
/// # Arguments
///
/// * `$msg` - A Message.
///
/// # Example
/// ```ignore
///   let msg1 = physim_core::msg!(self,"topic","message",MessagePriority::Low);
///   post_bus_msg!(msg1);
/// ```
///
/// # Safety
///
/// This macro internally uses `unsafe` to call a C-style function with a raw pointer.
/// It assumes that the global target has been correctly set and remains valid for
/// the duration of the program.
///
/// # Panics
///
/// Does not panic, but invoking this macro before the target is set will likely lead
/// to undefined behavior (null pointer dereference).
#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! post_bus_msg {
    ($msg:expr) => {
        unsafe { physim_core::messages::callback(crate::GLOBAL_BUS_TARGET, $msg.to_c_message()) }
    };
}

/// Loads a plugin library and sets its global callback target to a shared message bus.
///
/// This function dynamically loads a plugin (shared library) from the given `path` using `libloading`,
/// looks up the `set_callback_target` symbol, and passes a raw pointer to the provided `MessageBus`
/// (wrapped in `Arc<Mutex<_>>`) to the plugin.
///
/// This effectively registers the bus as the target for messages from the plugin via a globally stored pointer.
///
/// # Arguments
///
/// * `path` - The file system path to the compiled plugin dynamic library (e.g., `.so`, `.dll`, `.dylib`).
/// * `bus` - A reference-counted, thread-safe `MessageBus` wrapped in a `Mutex` for synchronization.
///
/// # Returns
///
/// Returns `Ok(())` if the library was successfully loaded and the callback target was set.
/// Returns an error if the library could not be loaded or the `set_callback_target` symbol was not found.
///
/// # Safety
///
/// This function is `unsafe` for several reasons:
/// - It performs raw pointer casting from `Arc<Mutex<MessageBus>>` to `*mut c_void`.
/// - It assumes the dynamic library at `path` is trusted and that the `set_callback_target` function
///   has the correct signature and behavior.
/// - The caller must ensure that the `bus` outlives any use of the raw pointer in the plugin.
///
/// Misuse can lead to undefined behavior if the pointer is invalidated or misinterpreted.
///
/// # Errors
///
/// This function will return an error if:
/// - The dynamic library cannot be opened at the given path.
/// - The `set_callback_target` symbol is missing or has an incompatible signature.
pub unsafe fn set_bus(
    element: &RegisteredElement,
    bus: Arc<Mutex<MessageBus>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = libloading::Library::new(&element.lib_path)?;
    let set_target: libloading::Symbol<unsafe extern "C" fn(*mut core::ffi::c_void)> =
        lib.get(b"set_callback_target")?;
    let bus_raw_ptr = Arc::into_raw(bus) as *mut core::ffi::c_void;
    set_target(bus_raw_ptr);
    Ok(())
}

type SetupLogger = extern "Rust" fn(
    logger: &'static dyn log::Log,
    level: log::LevelFilter,
) -> Result<(), log::SetLoggerError>;

pub fn setup_plugin_logger(element: &RegisteredElement) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let lib = libloading::Library::new(&element.lib_path)?;
        let ret = lib.get::<SetupLogger>(b"setup_logger");
        if let Ok(setup_logger) = ret {
            // I think it's basically fine to ignore this error. the global logger
            // pointer can only be set once, but the the plugins need to also use
            // the setup function to work.
            let _ = setup_logger(log::logger(), log::max_level());
        }
    }
    Ok(())
}

// determined at run time
#[derive(Debug)]
pub struct RegisteredElement {
    element_info: ElementMeta,
    pub lib_path: String,
    properties: HashMap<String, String>,
}

impl RegisteredElement {
    fn new(element_info: ElementMeta, lib_path: &str, properties: HashMap<String, String>) -> Self {
        RegisteredElement {
            element_info,
            lib_path: lib_path.to_string(),
            properties,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.element_info.name
    }

    pub fn get_element_kind(&self) -> &ElementKind {
        &self.element_info.kind
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
            ElementKind::Synth => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_red(),
                "synth".red().dim()
            ),
            ElementKind::Transmute => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_white(),
                "transmute".white().dim()
            ),
            ElementKind::Integrator => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_blue(),
                "integrator".blue().dim()
            ),
        }
    }

    pub fn print_element_info_verbose(&self) {
        println!("{}", "Overview".bold().underline().bright_blue());
        println!("{:>10} - {}", "Name".bold(), self.element_info.name.green());
        println!(
            "{:>10} - {}",
            "Blurb".bold(),
            self.element_info.blurb.green()
        );
        println!(
            "{:>10} - {:#?}",
            "Kind".bold(),
            self.element_info.kind.green()
        );
        if !self.properties.is_empty() {
            println!();
            println!("{}", "Properties".underline().bold().bright_blue());
        }
        for (key, desc) in self.properties.iter() {
            println!("{:>10} - {}", key.bold(), desc.green());
        }
        println!();
        println!("{}", "Meta data".underline().bold().bright_blue());
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
        println!(
            "{:>10} - {}",
            "Repository".bold(),
            self.element_info.repo.green()
        );
        println!();
        println!(
            "Loaded from the {} plugin located in {}",
            self.element_info.plugin.green(),
            self.lib_path.green()
        );
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
                        if let Ok(get_plugin_abi_info) = lib.get::<libloading::Symbol<
                            unsafe extern "C" fn() -> std::ffi::CString,
                        >>(
                            b"get_plugin_abi_info"
                        ) {
                            let rust_version = get_plugin_abi_info().into_string().unwrap();
                            if rust_version != PHYSIM_PLUGIN_LOADER_RUSTC_VERSION {
                                eprintln!("{} was built with a different version of the rust compiler or for a different platform. The plugin compiled with {} but physim compiled with {} ",&lib_path, rust_version,PHYSIM_PLUGIN_LOADER_RUSTC_VERSION);
                            }
                        } else {
                            continue;
                        }

                        if let Ok(register_plugin) = lib.get::<libloading::Symbol<
                            unsafe extern "C" fn() -> std::ffi::CString,
                        >>(
                            b"register_plugin"
                        ) {
                            let els = register_plugin().into_string().unwrap();
                            for el in els.split(",") {
                                let register_element =
                                        lib.get::<libloading::Symbol<
                                            unsafe extern "C" fn() -> ElementMeta,
                                        >>(
                                            format!("{el}_register").as_bytes()
                                        )
                                        .unwrap();
                                let element_info = register_element();

                                let properties = match element_info.kind {
                                    ElementKind::Transform => {
                                        let el = TransformElementHandler::load(
                                            &lib_path,
                                            &element_info.name,
                                            HashMap::new(),
                                        )
                                        .unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                    _ => {
                                        let el: Arc<MetaElement> = Loadable::load(
                                            &lib_path,
                                            &element_info.name,
                                            HashMap::new(),
                                        )
                                        .unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                };

                                elements.push(RegisteredElement::new(
                                    element_info,
                                    &lib_path,
                                    properties,
                                ));
                            }
                        }
                    };
                }
            }
        }
    }
    elements
}

/// Struct for determining metadata
struct MetaElement {
    instance: Box<dyn Element>,
}
impl Loadable for MetaElement {
    type Item = Box<dyn Element>;

    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl Element for MetaElement {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions()
    }
}

impl MessageClient for MetaElement {}

pub fn discover_map() -> HashMap<String, RegisteredElement> {
    let elements = discover();
    for element in &elements {
        if setup_plugin_logger(element).is_err() {
            eprintln!("Plugin doesn't implement setup_logger");
        };
    }

    elements
        .into_iter()
        .map(|m| (m.element_info.name.clone(), m))
        .collect()
}
