use std::{collections::HashMap, sync::Mutex};

use physim_attribute::integrator_element;
use physim_core::{
    Entity,
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
        forces: &[physim_core::Force],
        dt: f32,
    ) {
        self.previous_state = entities.to_vec();
        for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
            let m = entity.mass;
            let a = [f.fx / m, f.fy / m, f.fz / m];
            let x = entity.x + entity.vx * dt + 0.5 * a[0] * (dt.powi(2));
            let y = entity.y + entity.vy * dt + 0.5 * a[1] * (dt.powi(2));
            let z = entity.z + entity.vz * dt + 0.5 * a[2] * (dt.powi(2));

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

    fn integration(
        &mut self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        forces: &[physim_core::Force],
        dt: f32,
    ) {
        self.previous_state = entities.to_vec();
        for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
            let prev = self.previous_state.get(idx).unwrap();
            let m = entity.mass;
            let a = [f.fx / m, f.fy / m, f.fz / m];
            let x = 2_f32 * entity.x - prev.x + a[0] * (dt.powi(2));
            let y = 2_f32 * entity.y - prev.y + a[1] * (dt.powi(2));
            let z = 2_f32 * entity.z - prev.z + a[2] * (dt.powi(2));

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
    }
}

impl IntegratorElement for Verlet {
    fn integrate(
        &self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        forces: &[physim_core::Force],
        dt: f32,
    ) {
        let mut inner = self.inner.lock().unwrap();
        if inner.previous_state.len() != entities.len() {
            inner.initial_integration(entities, new_state, forces, dt);
        } else {
            inner.integration(entities, new_state, forces, dt);
        }
    }

    fn get_steps(&self) -> usize {
        1
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
