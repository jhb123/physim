use std::{collections::HashMap, error::Error, sync::mpsc::Receiver};

use crate::{messages::MessageClient, Entity};

use super::Element;

pub trait RenderElement: Element + Send + Sync + MessageClient {
    fn render(&self, state_recv: Receiver<Vec<Entity>>);
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
    pub fn render(&self, state_recv: Receiver<Vec<Entity>>) {
        self.instance.render(state_recv);
    }
}

impl Element for RenderElementHandler {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions()
    }
}

impl MessageClient for RenderElementHandler {
    fn recv_message(&self, message: &crate::messages::Message) {
        self.instance.recv_message(message)
    }
    fn post_configuration_messages(&self) {
        self.instance.post_configuration_messages();
    }
}
