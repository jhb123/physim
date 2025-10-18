use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use libloading::{Library, Symbol};
use once_cell::sync::OnceCell;
use serde_json::Value;

use crate::messages::{MessageBus, MessageClient};

pub mod generator;
pub mod integrator;
pub mod meta;
pub mod render;
pub mod transform;
pub mod transmute;
pub mod deps {
    pub use ::serde_json;
}

mod discover;

pub use discover::{element_db, RegisteredElement};
pub use meta::*;

static LIBRARY_LOADER: OnceCell<LibLoader> = OnceCell::new();

#[derive(Default, Debug)]
struct LibLoader {
    libraries: Mutex<HashMap<String, Arc<Library>>>,
}

impl LibLoader {
    fn initialise() -> &'static Self {
        LIBRARY_LOADER.get_or_init(|| Self {
            libraries: Mutex::new(HashMap::new()),
        })
    }

    pub unsafe fn get(path: &str) -> Result<Arc<Library>, libloading::Error> {
        let lib_loader = LibLoader::initialise();
        let mut libraries = match lib_loader.libraries.lock() {
            Ok(guard) => guard,
            Err(_) => {
                eprintln!("Fatal: library cache lock poisoned. Exiting.");
                std::process::exit(1);
            }
        };
        if let Some(lib) = libraries.get(path) {
            return Ok(Arc::clone(lib));
        }
        let lib = Arc::new(unsafe { Library::new(path)? });
        libraries.insert(path.to_string(), Arc::clone(&lib));
        Ok(lib)
    }
}

#[derive(Debug, Copy, Clone)]
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
) -> Result<T, libloading::Error> {
    let fn_name = format!("{name}_create_element");
    let lib = LibLoader::get(path)?;
    type GetNewFnType<T> = unsafe extern "Rust" fn(HashMap<String, Value>) -> T;

    let get_new_fn: Symbol<GetNewFnType<T>> = lib.get(fn_name.as_bytes())?;
    Ok(get_new_fn(properties))
}

pub trait ElementCreator {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self>;
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

type RegisterPluginFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

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
        unsafe {
            physim_core::messages::post_bus_callback(crate::GLOBAL_BUS_TARGET, $msg.to_c_message())
        }
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
) -> Result<(), libloading::Error> {
    let lib = LibLoader::get(element.get_lib_path())?;
    let set_target: Symbol<unsafe extern "C" fn(*mut core::ffi::c_void)> =
        lib.get(b"set_callback_target")?;
    let bus_raw_ptr = Arc::into_raw(bus) as *mut core::ffi::c_void;
    set_target(bus_raw_ptr);
    Ok(())
}

type SetupLogger = extern "Rust" fn(
    logger: &'static dyn log::Log,
    level: log::LevelFilter,
) -> Result<(), log::SetLoggerError>;

pub fn setup_plugin_logger(element: &RegisteredElement) -> Result<(), libloading::Error> {
    unsafe {
        let lib = LibLoader::get(element.get_lib_path())?;
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
