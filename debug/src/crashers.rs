use std::collections::HashMap;

use physim_attribute::transform_element;
use physim_core::{
    Acceleration, Entity, messages::MessageClient, plugin::transform::TransformElement,
};
use serde_json::Value;

#[transform_element(
    name = "crashtransform",
    blurb = "causes a crash in the transform part of pipeline"
)]
pub struct CrashTransform {}

impl TransformElement for CrashTransform {
    fn transform(&self, data: &[Entity], accelerations: &mut [Acceleration]) {
        let a = data[1000000000000000];
        accelerations[0] += Acceleration {
            x: a.x,
            y: a.y,
            z: a.z,
        };
    }

    fn new(_properties: HashMap<String, Value>) -> Self {
        // panic!("oh dear!");
        CrashTransform {}
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        panic!("oh no!")
    }
}

impl MessageClient for CrashTransform {}

// impl Drop for CrashTransform {
//     fn drop(&mut self) {
//         todo!("Lets see what happens!")
//     }
// }
