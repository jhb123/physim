use std::{
    collections::HashMap,
    error::Error,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};

use serde_json::Value;

use crate::{
    messages::{CMessage, MessageClient},
    plugin::{host_alloc_string, LibLoader},
    Acceleration, Entity,
};

use super::Element;

#[derive(Debug)]
pub enum TransformElementLoadError {
    DylibError(libloading::Error),
    NullElement,
}

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
    pub get_property_descriptions: unsafe extern "C" fn(
        *mut std::ffi::c_void,
        crate::plugin::RustStringAllocFn,
    ) -> *mut std::ffi::c_char,
    pub recv_message:
        unsafe extern "C" fn(obj: *mut std::ffi::c_void, msg: *const crate::messages::CMessage),
    pub post_configuration_messages: unsafe extern "C" fn(obj: *mut std::ffi::c_void),
}

pub struct TransformElementHandler {
    api: &'static TransformElementAPI,
    instance: AtomicPtr<std::ffi::c_void>,
}

impl TransformElementHandler {
    pub fn load(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<Arc<Self>, TransformElementLoadError> {
        unsafe {
            let api_fn_name = format!("{name}_get_api");
            let properties = serde_json::to_string(&properties)
                .expect("serde::Value and String can definely be serialised");
            let lib = LibLoader::get(path).map_err(TransformElementLoadError::DylibError)?;
            let get_api: libloading::Symbol<unsafe extern "C" fn() -> *const TransformElementAPI> =
                lib.get(api_fn_name.as_bytes())
                    .map_err(TransformElementLoadError::DylibError)?;
            let api = get_api();
            let (c, u, _l) = properties.into_raw_parts();
            let instance = ((*api).init)(c, u);
            if instance.is_null() {
                return Err(TransformElementLoadError::NullElement);
            }
            let element = Arc::new(Self {
                api: &*api,
                instance: AtomicPtr::new(instance),
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
        let value = unsafe {
            (self.api.get_property_descriptions)(
                self.instance.load(Ordering::SeqCst),
                host_alloc_string,
            )
        };
        if value.is_null() {
            return Err("Unable to load descriptions of properties".into());
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
    fn recv_message(&self, message: &crate::messages::Message) {
        let c_message = message.to_c_message();
        unsafe {
            (self.api.recv_message)(
                self.instance.load(Ordering::SeqCst),
                &c_message as *const CMessage,
            )
        }
        c_message.to_message();
    }

    fn post_configuration_messages(&self) {
        unsafe { (self.api.post_configuration_messages)(self.instance.load(Ordering::SeqCst)) }
    }
}
