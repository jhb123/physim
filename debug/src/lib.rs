#![feature(str_from_raw_parts)]
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use physim_attribute::{
    initialise_state_element, render_element, synth_element, transform_element,
};
use physim_core::{
    Entity,
    messages::{MessageClient, MessagePriority},
    msg,
    plugin::{
        Element, ElementCreator, generator::GeneratorElement, render::RenderElement,
        transform::TransformElement,
    },
    post_bus_msg, register_plugin,
};
use rand::Rng;
use serde_json::Value;

register_plugin!("randsynth", "debug", "fakesink", "msgdebug");

#[synth_element(name = "randsynth", blurb = "Generate a random entity")]
struct RandSynth {
    active: AtomicBool,
}

impl ElementCreator for RandSynth {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            active: AtomicBool::new(false),
        })
    }
}

impl GeneratorElement for RandSynth {
    fn create_entities(&self) -> Vec<Entity> {
        if self.active.load(Ordering::Relaxed) {
            let e = Entity::new2(
                rand::rng().random_range(-1.0..1.0),
                rand::rng().random_range(-1.0..1.0),
                rand::rng().random_range(-1.0..1.0),
                f32::log10(rand::rng().random_range(1.0..6.0)),
                0.03,
            );
            vec![e]
        } else {
            vec![]
        }
    }
}

impl Element for RandSynth {
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

impl MessageClient for RandSynth {
    fn recv_message(&self, message: physim_core::messages::Message) {
        if &message.topic == "keyboard.press" {
            match message.message.as_str() {
                "t" => self.active.fetch_xor(true, Ordering::Relaxed),
                _ => false,
            };
        }
    }
}

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

impl MessageClient for DebugTransform {}

#[render_element(name = "fakesink", blurb = "Do nothing with data")]
struct FakeSink {
    state: Mutex<u64>,
}

impl ElementCreator for FakeSink {
    fn create_element(_properties: HashMap<String, Value>) -> Box<Self> {
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
}

impl Element for FakeSink {
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

#[initialise_state_element(name = "msgdebug", blurb = "Print messages")]
struct MessageDebug {}

impl ElementCreator for MessageDebug {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {})
    }
}

impl GeneratorElement for MessageDebug {
    fn create_entities(&self) -> Vec<Entity> {
        vec![]
    }
}

impl Element for MessageDebug {
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

impl MessageClient for MessageDebug {
    fn recv_message(&self, message: physim_core::messages::Message) {
        println!(
            "[MSGDEBUG] Priority: {:?} - topic {} - message: {}",
            message.priority, message.topic, message.message
        )
    }
}
