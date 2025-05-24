use std::{
    collections::HashMap, error::Error, ffi::c_void, str::FromStr, sync::{
        atomic::{AtomicPtr, Ordering}, Arc, Mutex
    }
};

use libloading::Library;
use serde_json::Value;

use crate::{messages::{CMessage, MessageBus, MessageClient}, Entity};

use super::generator::ElementConfigurationHandler;

pub trait TransformElement {
    fn new(properties: HashMap<String, Value>) -> Self;
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32);
    fn set_properties(&mut self, properties: HashMap<String, Value>);
    fn get_property(&mut self, prop: &str) -> Result<Value, Box<dyn Error>>;
    fn get_property_descriptions(&mut self) -> HashMap<String, String>;
}

#[repr(C)]
pub struct TransformElementAPI {
    pub init: unsafe extern "C" fn(*const u8, usize) -> *mut std::ffi::c_void,
    pub transform:
        unsafe extern "C" fn(*mut std::ffi::c_void, *const Entity, usize, *mut Entity, usize, f32),
    pub destroy: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub set_properties: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_char),
    pub get_property:
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_char) -> *mut std::ffi::c_char,
    pub get_property_descriptions:
        unsafe extern "C" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_char,
    pub recv_message: unsafe extern "C" fn(obj: *mut std::ffi::c_void, msg: *mut std::ffi::c_void)
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
    ) -> Result<Mutex<Self>, Box<dyn std::error::Error>> {
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
                return Err("Could not create_entities element".into());
            }
            Ok(Mutex::new(Self {
                api: &*api,
                instance: AtomicPtr::new(instance),
                _lib: lib,
            }))
        }
    }

    pub fn loadv2(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
        bus: Arc<Mutex<MessageBus>>
    ) -> Result<Mutex<Arc<Self>>, Box<dyn std::error::Error>> {
        // Mutex<Arc<Self>> looks weird, but it's for type coercion and it's easier!
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
                return Err("Could not create_entities element".into());
            }

            let set_target: libloading::Symbol<unsafe extern "C" fn(*mut c_void)> =
                        lib.get(b"set_callback_target").unwrap();
            
            let bus_raw_ptr = Arc::into_raw(bus) as *mut c_void;
            set_target(bus_raw_ptr);

            Ok(Mutex::new(Arc::new(Self {
                api: &*api,
                instance: AtomicPtr::new(instance),
                _lib: lib,
            })))
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


impl ElementConfigurationHandler for TransformElementHandler {
    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        // covert hashmap into something else?
        let json = serde_json::to_string(&new_props).unwrap();
        let json = std::ffi::CString::new(json).unwrap().into_raw(); // danger!

        unsafe { (self.api.set_properties)(self.instance.load(Ordering::Relaxed), json) }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>> {
        let prop_ptr = std::ffi::CString::new(prop).unwrap().into_raw(); // danger!
        let value =
            unsafe { (self.api.get_property)(self.instance.load(Ordering::Relaxed), prop_ptr) };
        if value.is_null() {
            return Err(format!("{prop} is not a property").into());
        }
        let value = unsafe { std::ffi::CString::from_raw(value) };
        let v = value.to_str().map_err(Box::new)?;
        Ok(Value::from_str(v).map_err(Box::new)?)
    }

    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        let value =
            unsafe { (self.api.get_property_descriptions)(self.instance.load(Ordering::Relaxed)) };
        if value.is_null() {
            todo!()
            // return Err(format!("{prop} is not a property").into());
        }
        let value = unsafe { std::ffi::CString::from_raw(value) };
        let v = value.to_str().map_err(Box::new)?;
        Ok(serde_json::from_str(v).map_err(Box::new)?)
    }
}

impl Drop for TransformElementHandler {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl MessageClient for TransformElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        let (c_message,a,b) = message.to_c_message();
        let b = Box::new(c_message);
        let msg = Box::into_raw(b)as *mut core::ffi::c_void;
        unsafe { (self.api.recv_message)(self.instance.load(Ordering::Relaxed), msg ) }
    }
}
