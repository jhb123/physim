use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use physim_attribute::transform_element;
use physim_core::{Entity, Force, messages::MessageClient, plugin::transform::TransformElement};
use serde_json::Value;

#[transform_element(
    name = "impulse",
    blurb = "Apply an impulse force to all particles on the initial iteration of the simulation."
)]
pub struct Impluse {
    should_pulse: AtomicBool,
    force: Force,
}

impl TransformElement for Impluse {
    fn transform(&self, _state: &[Entity], forces: &mut [Force]) {
        if self.should_pulse.swap(false, Ordering::Relaxed) {
            for f in forces {
                *f += self.force;
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let fx: f32 = properties.get("fx").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let fy: f32 = properties.get("fy").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let fz: f32 = properties.get("fz").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let force = Force { fx, fy, fz };
        Impluse {
            force,
            should_pulse: AtomicBool::new(true),
        }
    }

    fn set_properties(&self, _properties: HashMap<String, Value>) {}

    fn get_property(&self, _prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                String::from("fx"),
                String::from("Force in x direction. Default=0.0"),
            ),
            (
                String::from("fy"),
                String::from("Force in y direction. Default=0.0"),
            ),
            (
                String::from("fz"),
                String::from("Force in z direction. Default=0.0"),
            ),
        ])
    }
}

impl MessageClient for Impluse {}
