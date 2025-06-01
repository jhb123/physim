#![feature(str_from_raw_parts)]
use std::{collections::HashMap, sync::Mutex};

use physim_attribute::{synth_element, transform_element};
use physim_core::{
    Entity,
    messages::{Message, MessageClient, MessagePriority},
    plugin::{
        generator::{GeneratorElement, GeneratorElementCreator},
        transform::TransformElement,
    },
    post_bus_msg, register_plugin,
};
use rand::Rng;
use serde_json::Value;

register_plugin!("randsynth", "debug");

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

#[transform_element(name = "debug", blurb = "Pass through data with no effect")]
pub struct DebugTransform {
    state: Mutex<u64>,
}

impl TransformElement for DebugTransform {
    fn transform(&self, state: &[Entity], new_state: &mut [Entity], _dt: f32) {
        for (i, e) in state.iter().enumerate() {
            new_state[i] = *e
        }

        // let msg1 = physim_core::msg!(
        //     self,
        //     "debugplugin",
        //     "this is a message from debug transform",
        //     MessagePriority::Low
        // );
        let msg1 = Message {
            topic: "debugplugin".to_string(),
            message: "$message".to_string(),
            priority: MessagePriority::Low,
            sender_id: self as *const Self as *const () as usize,
        };
        println!("Posting: {:?}", msg1);
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

// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn debug_transform_recv_message(obj: *mut std::ffi::c_void, msg: *mut std::ffi::c_void) {
//     if obj.is_null() {return };
//     let el: &mut DebugTransform = unsafe { &mut *(obj as *mut DebugTransform) };
//     let msg = unsafe { (*(obj as *mut Message)).clone() };
//     el.recv_message(msg);
// }
