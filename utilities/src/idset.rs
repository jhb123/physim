use std::{collections::HashMap, sync::atomic::AtomicU64};

use physim_attribute::transmute_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, transmute::TransmuteElement},
};
use serde_json::Value;

#[transmute_element(name = "idset", blurb = "Give unique IDs to elements")]
struct IdTransmute {
    current_id: AtomicU64,
}

impl TransmuteElement for IdTransmute {
    fn transmute(&self, data: &mut Vec<Entity>) {
        data.iter_mut().filter(|x| x.id == 0).for_each(|e| {
            e.id = self
                .current_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize;
        });
    }
}

impl ElementCreator for IdTransmute {
    fn create_element(_props: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            current_id: AtomicU64::new(1),
        })
    }
}

impl Element for IdTransmute {
    fn set_properties(&self, _new_props: HashMap<String, Value>) {}

    fn get_property(&self, _prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

impl MessageClient for IdTransmute {}
