use std::{collections::HashMap, error::Error};

use serde_json::Value;

use crate::{messages::MessageClient, plugin::Element, Entity};

pub trait GeneratorElement: Element + Send + Sync {
    fn create_entities(&self) -> Vec<Entity>;
}

pub struct GeneratorElementHandler {
    instance: Box<dyn GeneratorElement>,
}

impl super::Loadable for GeneratorElementHandler {
    type Item = Box<dyn GeneratorElement>;

    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl GeneratorElementHandler {
    pub fn create_entities(&self) -> Vec<Entity> {
        self.instance.create_entities()
    }
}

impl Element for GeneratorElementHandler {
    fn set_properties(&self, new_props: HashMap<String, Value>) {
        self.instance.set_properties(new_props);
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn Error>> {
        self.instance.get_property(prop)
    }

    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions()
    }
}

impl MessageClient for GeneratorElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        self.instance.recv_message(message)
    }
}
