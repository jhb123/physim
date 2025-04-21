// use std::{collections::HashMap, error::Error};

// use serde_json::Value;

// use crate::Entity;

// pub trait SynthElementCreator {
//     fn create_element(properties: HashMap<String, Value>) -> Box<dyn SynthElement>;
// }

// pub trait SynthElement {
//     fn create_entities(&self) -> Vec<Entity>;
//     fn set_properties(&mut self, new_props: HashMap<String, Value>);
//     fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>>;
//     fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>>;
// }
// pub struct SynthElementHandler {
//     instance: Box<dyn SynthElement>,
// }

// impl SynthElementHandler {
//     pub fn load(
//         path: &str,
//         name: &str,
//         properties: HashMap<String, Value>,
//     ) -> Result<SynthElementHandler, Box<dyn std::error::Error>> {
//         unsafe {
//             let fn_name = format!("{name}_create_element");
//             let lib = libloading::Library::new(path)?;
//             type GetNewFnType = unsafe extern "Rust" fn(
//                 properties: HashMap<String, Value>,
//             )
//                 -> Box<dyn SynthElement>;
//             let get_new_fn: libloading::Symbol<GetNewFnType> = lib.get(fn_name.as_bytes())?;
//             let ins = get_new_fn(properties);
//             Ok(SynthElementHandler { instance: ins })
//         }
//     }

//     pub fn create_entities(&self) -> Vec<Entity> {
//         self.instance.create_entities()
//     }
// }

// impl ElementConfigurationHandler for SynthElementHandler {
//     fn set_properties(&mut self, new_props: HashMap<String, Value>) {
//         self.instance.set_properties(new_props);
//     }

//     fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>> {
//         self.instance.get_property(prop)
//     }

//     fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
//         self.instance.get_property_descriptions()
//     }
// }

// pub trait ElementConfigurationHandler {
//     fn set_properties(&mut self, new_props: HashMap<String, Value>);
//     fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>>;
//     fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>>;
// }
