use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use physim_attribute::transform_element;
use physim_core::{
    Acceleration, Entity, messages::MessageClient, plugin::transform::TransformElement,
};
use serde_json::Value;

#[transform_element(
    name = "impulse",
    blurb = "Apply an impulse acceleration to all particles on the initial iteration of the simulation."
)]
pub struct Impluse {
    should_pulse: AtomicBool,
    acceleration: Acceleration,
}

impl TransformElement for Impluse {
    fn transform(&self, _state: &[Entity], accelerations: &mut [Acceleration]) {
        if self.should_pulse.swap(false, Ordering::Relaxed) {
            for a in accelerations {
                *a += self.acceleration;
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let x = properties.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let y = properties.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let z = properties.get("z").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let acceleration = Acceleration { x, y, z };
        Impluse {
            acceleration,
            should_pulse: AtomicBool::new(true),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                String::from("x"),
                String::from("Acceleration in x direction. Default=0.0"),
            ),
            (
                String::from("y"),
                String::from("Acceleration in y direction. Default=0.0"),
            ),
            (
                String::from("z"),
                String::from("Acceleration in z direction. Default=0.0"),
            ),
        ])
    }
}

impl MessageClient for Impluse {}
