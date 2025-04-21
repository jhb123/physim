#![feature(str_from_raw_parts)]
use std::collections::HashMap;

use physim_attribute::{synth_element, transform_element};
use physim_core::{
    plugin::{generator::{GeneratorElement, GeneratorElementCreator}, transform::TransformElement}, register_plugin, Entity
};
use rand::Rng;
use serde_json::Value;

register_plugin!("randsynth","debug");

#[synth_element(name = "randsynth", blurb = "Generate a random entity")]
struct RandSynth {}

impl GeneratorElementCreator for RandSynth {
    fn create_element(_: HashMap<String, Value>) -> Box<dyn GeneratorElement> {
        Box::new(Self {})
    }
}

impl GeneratorElement for RandSynth {
    fn create_entities(&self) -> Vec<Entity> {
        let e = Entity::new2(
            rand::rng().random_range(-1.0..1.0),
            rand::rng().random_range(-1.0..1.0),
            rand::rng().random_range(-1.0..1.0),
            f32::log10(rand::rng().random_range(1.0..6.0)),
            0.03,
        );
        vec![e]
    }

    fn set_properties(&mut self, _: HashMap<String, Value>) {}

    fn get_property(&self, _: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

#[transform_element(name = "debug", blurb = "Pass through data with no effect")]
pub struct DebugTransform {
    state: u64,
}

impl TransformElement for DebugTransform {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], _dt: f32) {
        for (i, e) in state.iter().enumerate() {
            new_state[i] = *e
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        DebugTransform {
            state: properties
                .get("prop")
                .and_then(|x| x.as_u64())
                .unwrap_or_default(),
        }
    }

    fn set_properties(&mut self, properties: HashMap<String, Value>) {
        if let Some(state) = properties.get("state").and_then(|state| state.as_u64()) {
            self.state = state
        }
    }

    fn get_property(&mut self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "state" => Ok(Value::Number(self.state.into())),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&mut self) -> HashMap<String, String> {
        HashMap::new()
    }
}
