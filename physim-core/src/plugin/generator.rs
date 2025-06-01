use std::{collections::HashMap, error::Error};

use serde_json::Value;

use crate::{messages::MessageClient, Entity};

pub trait GeneratorElementCreator {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn GeneratorElement>;
}

pub trait GeneratorElement: Send + Sync {
    fn create_entities(&self) -> Vec<Entity>;
    fn set_properties(&self, new_props: HashMap<String, Value>);
    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>>;
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>>;
}

pub struct GeneratorElementHandler {
    instance: Box<dyn GeneratorElement>,
}

impl GeneratorElementHandler {
    pub fn load(
        path: &str,
        name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<GeneratorElementHandler, Box<dyn std::error::Error>> {
        unsafe {
            let fn_name = format!("{name}_create_element");
            let lib = libloading::Library::new(path)?;
            type GetNewFnType = unsafe extern "Rust" fn(
                properties: HashMap<String, Value>,
            )
                -> Box<dyn GeneratorElement + Send>;
            let get_new_fn: libloading::Symbol<GetNewFnType> = lib.get(fn_name.as_bytes())?;
            let ins = get_new_fn(properties);
            Ok(GeneratorElementHandler { instance: ins })
        }
    }

    pub fn create_entities(&self) -> Vec<Entity> {
        self.instance.create_entities()
    }
}

impl ElementConfigurationHandler for GeneratorElementHandler {
    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        self.instance.set_properties(new_props);
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>> {
        self.instance.get_property(prop)
    }

    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions()
    }
}

pub trait ElementConfigurationHandler {
    fn set_properties(&mut self, new_props: HashMap<String, Value>);
    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>>;
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>>;
}

impl MessageClient for GeneratorElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        let c_message = message.to_c_message();
        let b = Box::new(c_message);
        let msg = Box::into_raw(b) as *mut core::ffi::c_void;
        todo!("implement the recv message stuff in macros")
        // unsafe { (self.api.recv_message)(self.instance.load(Ordering::Relaxed), msg) }
    }
}
