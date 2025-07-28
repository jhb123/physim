use std::{collections::HashMap, error::Error, sync::mpsc::Receiver};

use serde_json::Value;

use crate::{Entity, UniverseConfiguration, messages::MessageClient};

use super::Element;

pub trait RenderElement: Element + Send + Sync + MessageClient {
    fn render(&self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>);
}
pub struct RenderElementHandler {
    instance: Box<dyn RenderElement>,
}

impl super::Loadable for RenderElementHandler {
    type Item = Box<dyn RenderElement>;

    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl RenderElementHandler {
    pub fn render(&self, config: UniverseConfiguration, state_recv: Receiver<Vec<Entity>>) {
        self.instance.render(config, state_recv);
    }
}

impl Element for RenderElementHandler {
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

impl MessageClient for RenderElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        self.instance.recv_message(message)
    }
}
