#![feature(str_from_raw_parts)]
use std::{collections::HashMap, sync::Mutex};

use physim_attribute::{render_element, synth_element, transform_element};
use physim_core::{
    Entity,
    messages::{MessageClient, MessagePriority},
    msg,
    plugin::{
        generator::{GeneratorElement, GeneratorElementCreator},
        render::{RenderElement, RenderElementCreator},
        transform::TransformElement,
    },
    post_bus_msg, register_plugin,
};
use rand::Rng;
use serde_json::Value;

register_plugin!("randsynth", "debug", "fakesink");

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

    fn set_properties(&self, _: HashMap<String, Value>) {}

    fn get_property(&self, _: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

impl MessageClient for RandSynth {}

#[transform_element(name = "debug", blurb = "Pass through data with no effect")]
pub struct DebugTransform {
    state: Mutex<u64>,
}

impl TransformElement for DebugTransform {
    fn transform(&self, state: &[Entity], new_state: &mut [Entity], _dt: f32) {
        for (i, e) in state.iter().enumerate() {
            new_state[i] = *e
        }

        let msg1 = msg!(self, "debugplugin", "transformed", MessagePriority::Low);
        post_bus_msg!(msg1);
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        DebugTransform {
            state: Mutex::new(
                properties
                    .get("prop")
                    .and_then(|x| x.as_u64())
                    .unwrap_or_default(),
            ),
        }
    }

    fn set_properties(&self, properties: HashMap<String, Value>) {
        if let Some(state) = properties.get("state").and_then(|state| state.as_u64()) {
            *self.state.lock().unwrap() = state
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "state" => Ok(Value::Number((*self.state.lock().unwrap()).into())),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

// impl MessageClient for DebugTransform{}

impl MessageClient for DebugTransform {
    fn recv_message(&self, message: physim_core::messages::Message) {
        let sender_id = self as *const Self as *const () as usize;
        if message.sender_id == sender_id {
            print!(" FILTERED --> ")
        }
        println!(
            "Custom message:: Priority: {:?} - topic {} - message: {} - sender: {sender_id}",
            message.priority, message.topic, message.message
        )
    }
}

#[render_element(name = "fakesink", blurb = "Do nothing with data")]
struct FakeSink {
    state: Mutex<u64>,
}

impl RenderElementCreator for FakeSink {
    fn create_element(
        _properties: HashMap<String, Value>,
    ) -> Box<dyn physim_core::plugin::render::RenderElement> {
        Box::new(FakeSink {
            state: Mutex::new(0),
        })
    }
}

impl RenderElement for FakeSink {
    fn render(
        &self,
        _config: physim_core::UniverseConfiguration,
        state_recv: std::sync::mpsc::Receiver<Vec<Entity>>,
    ) {
        while state_recv.recv().is_ok() {
            println!("Rendering!");
        }
    }

    fn set_properties(&self, new_props: HashMap<String, Value>) {
        if let Some(state) = new_props.get("state").and_then(|state| state.as_u64()) {
            *self.state.lock().unwrap() = state
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "state" => Ok(Value::Number((*self.state.lock().unwrap()).into())),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::new())
    }
}

impl MessageClient for FakeSink {}
