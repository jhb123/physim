use std::collections::HashMap;

use physim_attribute::integrator_element;
use physim_core::{
    Acceleration,
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
        acc_fn: &dyn Fn(&[physim_core::Entity], &mut [Acceleration]),
        dt: f64,
    ) {
        let mut accelerations = vec![Acceleration::zero(); entities.len()];
        acc_fn(entities, &mut accelerations);

        for (idx, (entity, a)) in entities.iter().zip(accelerations).enumerate() {
            // S = s0 + ut + 1/2 a t^2
            let x = entity.x + entity.vx * dt + 0.5 * a.x * (dt.powi(2));
            let y = entity.y + entity.vy * dt + 0.5 * a.y * (dt.powi(2));
            let z = entity.z + entity.vz * dt + 0.5 * a.z * (dt.powi(2));

            // v = v0 +
            let vx = entity.vx + a.x * dt;
            let vy = entity.vy + a.y * dt;
            let vz = entity.vz + a.z * dt;

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
}

impl MessageClient for Euler {}

impl ElementCreator for Euler {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {})
    }
}

impl Element for Euler {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}
