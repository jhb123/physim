use std::{collections::HashMap, sync::Mutex};

use physim_attribute::integrator_element;
use physim_core::{
    Acceleration, Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, integrator::IntegratorElement},
};
use serde_json::Value;

#[integrator_element(
    name = "verlet",
    blurb = "Evaluate evolution with time using Verlet integration"
)]
struct Verlet {
    inner: Mutex<VerletInner>,
}

struct VerletInner {
    previous_state: Vec<Entity>,
}

impl VerletInner {
    fn initial_integration(
        &mut self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        accelerations: &[physim_core::Acceleration],
        dt: f64,
    ) {
        self.previous_state = entities.to_vec();
        for (idx, (entity, a)) in entities.iter().zip(accelerations).enumerate() {
            let x = entity.x + entity.vx * dt + 0.5 * a.x * (dt.powi(2));
            let y = entity.y + entity.vy * dt + 0.5 * a.y * (dt.powi(2));
            let z = entity.z + entity.vz * dt + 0.5 * a.z * (dt.powi(2));

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

    fn integration(
        &mut self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        accelerations: &[physim_core::Acceleration],
        dt: f64,
    ) {
        for (idx, (entity, a)) in entities.iter().zip(accelerations).enumerate() {
            let prev = self.previous_state.get(idx).unwrap();
            let x = 2_f64 * entity.x - prev.x + a.x * (dt.powi(2));
            let y = 2_f64 * entity.y - prev.y + a.y * (dt.powi(2));
            let z = 2_f64 * entity.z - prev.z + a.z * (dt.powi(2));

            let vx = (x - entity.x) / dt;
            let vy = (y - entity.y) / dt;
            let vz = (z - entity.z) / dt;

            let mut new_entity = *entity;
            new_entity.x = x;
            new_entity.y = y;
            new_entity.z = z;
            new_entity.vx = vx;
            new_entity.vy = vy;
            new_entity.vz = vz;
            new_state[idx] = new_entity;
        }
        self.previous_state = entities.to_vec();
    }
}

impl IntegratorElement for Verlet {
    fn integrate(
        &self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        acc_fn: &dyn Fn(&[Entity], &mut [Acceleration]),
        dt: f64,
    ) {
        let mut accelerations = vec![Acceleration::zero(); entities.len()];
        acc_fn(entities, &mut accelerations);
        let mut inner = self.inner.lock().unwrap();
        if inner.previous_state.len() != entities.len() {
            inner.initial_integration(entities, new_state, &accelerations, dt);
        } else {
            inner.integration(entities, new_state, &accelerations, dt);
        }
    }
}

impl MessageClient for Verlet {}

impl ElementCreator for Verlet {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        let inner = VerletInner {
            previous_state: vec![],
        };
        Box::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

impl Element for Verlet {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}
