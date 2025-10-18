use crate::{messages::MessageClient, plugin::Element, Entity};
use std::{collections::HashMap, error::Error};

pub trait GeneratorElement: Element + Send + Sync + private::Sealed {
    fn create_entities(&self) -> Vec<Entity>;
}

mod private {
    use crate::Entity;

    pub trait Sealed {
        fn create_entities_wrapped(&self) -> Vec<Entity>;
    }

    impl<T: super::GeneratorElement> Sealed for T {
        fn create_entities_wrapped(&self) -> Vec<Entity> {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.create_entities()))
            {
                Ok(state) => return state,
                Err(_) => {
                    eprintln!("Exiting...");
                    std::process::exit(1)
                }
            }
        }
    }
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
        self.instance.create_entities_wrapped()
    }
}

impl Element for GeneratorElementHandler {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions_wrapped()
    }
}

impl MessageClient for GeneratorElementHandler {
    fn recv_message(&self, message: &crate::messages::Message) {
        self.instance.recv_message(message)
    }
    fn post_configuration_messages(&self) {
        self.instance.post_configuration_messages();
    }
}
