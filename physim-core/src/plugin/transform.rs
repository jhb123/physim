use std::{
    collections::HashMap,
    error::Error,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};

use libloading::Library;
use serde_json::Value;

use crate::{messages::MessageClient, Acceleration, Entity};

use super::Element;

pub trait TransformElement: Send + Sync {
    fn new(properties: HashMap<String, Value>) -> Self;
    fn transform(&self, state: &[Entity], acceleration: &mut [Acceleration]);
    fn get_property_descriptions(&self) -> HashMap<String, String>;
}

#[repr(C)]
pub struct TransformElementAPI {
    pub init: unsafe extern "C" fn(*const u8, usize) -> *mut std::ffi::c_void,
    pub transform: unsafe extern "C" fn(
        *const std::ffi::c_void,
        *const Entity,
        usize,
        *mut Acceleration,
        usize,
    ),
    pub destroy: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub get_property_descriptions:
        unsafe extern "C" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_char,
    pub recv_message: unsafe extern "C" fn(obj: *mut std::ffi::c_void, msg: *mut std::ffi::c_void),
    pub post_configuration_messages: unsafe extern "C" fn(obj: *mut std::ffi::c_void),
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
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
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
            let element = Arc::new(Self {
                api: &*api,
                instance: AtomicPtr::new(instance),
                _lib: lib,
            });
            Ok(element)
        }
    }

    pub fn transform(&self, state: &[Entity], acceleration: &mut [Acceleration]) {
        let state_len = state.len();
        let state = state.as_ptr();
        let acceleration_len = state_len;
        let acceleration_ptr = acceleration.as_mut_ptr();
        let instance = self.instance.load(Ordering::SeqCst);
        if instance.is_null() {
            eprintln!("Transform is not loaded");
        } else {
            unsafe {
                (self.api.transform)(
                    instance,
                    state,
                    state_len,
                    acceleration_ptr,
                    acceleration_len,
                );
            }
            // new_state = std::slice::from_raw_parts_mut(new_state_ptr, new_state_len) ;
        }
    }

    pub fn destroy(&self) {
        unsafe {
            (self.api.destroy)(self.instance.load(Ordering::SeqCst));
        }
    }
}

impl Element for TransformElementHandler {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        let value =
            unsafe { (self.api.get_property_descriptions)(self.instance.load(Ordering::SeqCst)) };
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
        let c_message = message.to_c_message();
        let b = Box::new(c_message);
        let msg = Box::into_raw(b) as *mut core::ffi::c_void;
        unsafe { (self.api.recv_message)(self.instance.load(Ordering::SeqCst), msg) }
    }
    fn post_configuration_messages(&self) {
        unsafe { (self.api.post_configuration_messages)(self.instance.load(Ordering::SeqCst)) }
    }
}
