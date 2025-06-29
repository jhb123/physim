use crate::{messages::MessageClient, Entity, Force};

use super::Element;

pub trait IntegratorElement: Element + Send + Sync {
    fn integrate(&self, entities: &[Entity], new_state: &mut [Entity], forces: &[Force], dt: f32);
}

pub struct IntegratorElementHandler {
    instance: Box<dyn IntegratorElement>,
}

impl IntegratorElement for IntegratorElementHandler {
    fn integrate(&self, entities: &[Entity], new_state: &mut [Entity], forces: &[Force], dt: f32) {
        self.instance.integrate(entities, new_state, forces, dt);
    }
}

impl Element for IntegratorElementHandler {
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

impl super::Loadable for IntegratorElementHandler {
    type Item = Box<dyn IntegratorElement>;
    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl MessageClient for IntegratorElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        self.instance.recv_message(message);
    }
}
