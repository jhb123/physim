use std::collections::HashMap;

use physim_attribute::integrator_element;
use physim_core::{
    messages::MessageClient,
    plugin::{Element, ElementCreator, integrator::IntegratorElement},
};
use serde_json::Value;

#[integrator_element(
    name = "euler",
    blurb = "Evaluate evolution with time using Euler integration"
)]
struct Euler {}

impl IntegratorElement for Euler {
    fn integrate(
        &self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        forces: &[physim_core::Force],
        dt: f32,
    ) {
        for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
            let m = entity.mass;
            // f = ma
            let a = [f.fx / m, f.fy / m, f.fz / m];
            // S = s0 + ut + 1/2 a t^2
            let x = entity.x + entity.vx * dt + 0.5 * a[0] * (dt.powi(2));
            let y = entity.y + entity.vy * dt + 0.5 * a[1] * (dt.powi(2));
            let z = entity.z + entity.vz * dt + 0.5 * a[2] * (dt.powi(2));

            // v = v0 +
            let vx = entity.vx + a[0] * dt;
            let vy = entity.vy + a[1] * dt;
            let vz = entity.vz + a[2] * dt;

            let mut new_entity = *entity;
            new_entity.x = x;
            new_entity.y = y;
            new_entity.z = z;
            new_entity.vx = vx;
            new_entity.vy = vy;
            new_entity.vz = vz;
            new_state[idx] = new_entity;
        }
    }

    fn get_steps(&self) -> usize {
        1
    }
}

impl MessageClient for Euler {}

impl ElementCreator for Euler {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {})
    }
}

impl Element for Euler {
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
