use crate::{messages::MessageClient, Entity};

use super::Element;

pub trait TransmuteElement: Element + Send + Sync {
    fn transmute(&self, data: &mut Vec<Entity>);
}

pub struct TransmuteElementHandler {
    instance: Box<dyn TransmuteElement>,
}

impl TransmuteElement for TransmuteElementHandler {
    fn transmute(&self, data: &mut Vec<Entity>) {
        self.instance.transmute(data);
    }
}

impl Element for TransmuteElementHandler {
    fn get_property_descriptions(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        self.instance.get_property_descriptions()
    }
}

impl super::Loadable for TransmuteElementHandler {
    type Item = Box<dyn TransmuteElement>;
    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl MessageClient for TransmuteElementHandler {
    fn recv_message(&self, message: crate::messages::Message) {
        self.instance.recv_message(message);
    }
}
