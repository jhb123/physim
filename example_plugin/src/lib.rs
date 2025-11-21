#![feature(str_from_raw_parts)]
use std::collections::HashMap;

use serde_json::Value;

use physim_attribute::transform_element;
use physim_core::messages::MessageClient;
use physim_core::plugin::transform::TransformElement;
use physim_core::register_plugin;
use physim_core::{Acceleration, Entity};

// ANCHOR: element_declaration
register_plugin!("ex_drag");

#[transform_element(name = "ex_drag", blurb = "Applies a drag proportional to velocity")]
pub struct Drag {
    alpha: f64,
}
// ANCHOR_END: element_declaration

impl TransformElement for Drag {
    // ANCHOR: element_transform
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        for (acc, entity) in accelerations.iter_mut().zip(state) {
            *acc += Acceleration {
                x: -self.alpha * entity.vx * entity.vx.abs() / entity.mass,
                y: -self.alpha * entity.vy * entity.vy.abs() / entity.mass,
                z: -self.alpha * entity.vz * entity.vz.abs() / entity.mass,
            };
        }
    }
    // ANCHOR_END: element_transform
    // ANCHOR: element_props
    fn new(properties: HashMap<String, Value>) -> Self {
        Drag {
            alpha: properties
                .get("alpha")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([(String::from("alpha"), String::from("Coefficient of drag"))])
    }
    // ANCHOR_END: element_props
}

impl MessageClient for Drag {}
