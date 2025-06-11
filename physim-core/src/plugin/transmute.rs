use crate::{messages::MessageClient, Entity};

use super::{Element, ElementCreator};

/// For use by plugins
pub trait TransmuteElement: TransmuteElementInternal + ElementCreator {}

pub trait TransmuteElementInternal: Element + Send + Sync {
    fn transmute(&self, data: Vec<Entity>);
}

/// For use by pipeline
pub struct TransmuteElementHandler {
    instance: Box<dyn TransmuteElementInternal>,
}

impl TransmuteElementInternal for TransmuteElementHandler {
    fn transmute(&self, data: Vec<Entity>) {
        self.instance.transmute(data);
    }
}

impl Element for TransmuteElementHandler {
    fn set_properties(&self, new_props: std::collections::HashMap<String, serde_json::Value>) {
        self.instance.set_properties(new_props);
    }

    fn get_property(&self, prop: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.instance.get_property(prop)
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        self.instance.get_property_descriptions()
    }
}

impl super::Loadable for TransmuteElementHandler {
    type Item = Box<dyn TransmuteElementInternal>;
    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl MessageClient for TransmuteElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        self.instance.recv_message(message);
    }
}
