use std::{collections::HashMap, sync::Mutex};

use physim_attribute::integrator_element;
use physim_core::{
    Acceleration, Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, integrator::IntegratorElement},
};
use serde_json::Value;

#[integrator_element(
    name = "rk4",
    blurb = "Evaluate evolution with time using Rk4 integration"
)]

struct Rk4 {
    _inner: Mutex<InnerRk4>,
}

struct InnerRk4 {
    _step: u8,
}

impl IntegratorElement for Rk4 {
    fn integrate(
        &self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        acc_fn: &dyn Fn(&[Entity], &mut [Acceleration]),
        dt: f64,
    ) {
        let n = entities.len();

        // --- k1 ---
        let mut f1 = vec![Acceleration::zero(); n];
        acc_fn(entities, &mut f1);

        let k1: Vec<Entity> = entities
            .iter()
            .cloned()
            .zip(&f1)
            .map(|(e, a)| Entity {
                x: dt * e.vx,
                y: dt * e.vy,
                z: dt * e.vz,
                vx: dt * a.x,
                vy: dt * a.y,
                vz: dt * a.z,
                ..e
            })
            .collect();

        // --- k2 ---
        let temp_ents: Vec<Entity> = entities
            .iter()
            .cloned()
            .zip(&k1)
            .map(|(e, k)| Entity {
                x: e.x + 0.5 * k.x,
                y: e.y + 0.5 * k.y,
                z: e.z + 0.5 * k.z,
                vx: e.vx + 0.5 * k.vx,
                vy: e.vy + 0.5 * k.vy,
                vz: e.vz + 0.5 * k.vz,
                ..e
            })
            .collect();

        let mut f2 = vec![Acceleration::zero(); n];
        acc_fn(&temp_ents, &mut f2);

        let k2: Vec<Entity> = temp_ents
            .iter()
            .cloned()
            .zip(&f2)
            .map(|(e, a)| Entity {
                x: dt * e.vx,
                y: dt * e.vy,
                z: dt * e.vz,
                vx: dt * a.x,
                vy: dt * a.y,
                vz: dt * a.z,
                ..e
            })
            .collect();

        // --- k3 ---
        let temp_ents: Vec<Entity> = entities
            .iter()
            .cloned()
            .zip(&k2)
            .map(|(e, k)| Entity {
                x: e.x + 0.5 * k.x,
                y: e.y + 0.5 * k.y,
                z: e.z + 0.5 * k.z,
                vx: e.vx + 0.5 * k.vx,
                vy: e.vy + 0.5 * k.vy,
                vz: e.vz + 0.5 * k.vz,
                ..e
            })
            .collect();

        let mut f3 = vec![Acceleration::zero(); n];
        acc_fn(&temp_ents, &mut f3);

        let k3: Vec<Entity> = temp_ents
            .iter()
            .cloned()
            .zip(&f3)
            .map(|(e, a)| Entity {
                x: dt * e.vx,
                y: dt * e.vy,
                z: dt * e.vz,
                vx: dt * a.x,
                vy: dt * a.y,
                vz: dt * a.z,
                ..e
            })
            .collect();

        // --- k4 ---
        let temp_ents: Vec<Entity> = entities
            .iter()
            .cloned()
            .zip(&k3)
            .map(|(e, k)| Entity {
                x: e.x + k.x,
                y: e.y + k.y,
                z: e.z + k.z,
                vx: e.vx + k.vx,
                vy: e.vy + k.vy,
                vz: e.vz + k.vz,
                ..e
            })
            .collect();

        let mut f4 = vec![Acceleration::zero(); n];
        acc_fn(&temp_ents, &mut f4);

        let k4: Vec<Entity> = temp_ents
            .iter()
            .cloned()
            .zip(&f4)
            .map(|(e, a)| Entity {
                x: dt * e.vx,
                y: dt * e.vy,
                z: dt * e.vz,
                vx: dt * a.x,
                vy: dt * a.y,
                vz: dt * a.z,
                ..e
            })
            .collect();

        // --- Combine results ---
        for ((e, ns), (((k1, k2), k3), k4)) in entities
            .iter()
            .zip(new_state.iter_mut())
            .zip(k1.iter().zip(&k2).zip(&k3).zip(&k4))
        {
            ns.x = e.x + (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x) / 6.0;
            ns.y = e.y + (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y) / 6.0;
            ns.z = e.z + (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z) / 6.0;
            ns.vx = e.vx + (k1.vx + 2.0 * k2.vx + 2.0 * k3.vx + k4.vx) / 6.0;
            ns.vy = e.vy + (k1.vy + 2.0 * k2.vy + 2.0 * k3.vy + k4.vy) / 6.0;
            ns.vz = e.vz + (k1.vz + 2.0 * k2.vz + 2.0 * k3.vz + k4.vz) / 6.0;
            ns.mass = e.mass;
            ns.radius = e.radius;
            ns.id = e.id;
        }
    }
}

impl MessageClient for Rk4 {}

impl ElementCreator for Rk4 {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            _inner: Mutex::new(InnerRk4 { _step: 0 }),
        })
    }
}

impl Element for Rk4 {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}
