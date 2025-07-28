use std::{collections::HashMap, ops::Rem, sync::Mutex};

use physim_attribute::integrator_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, integrator::IntegratorElement},
};
use serde_json::Value;

#[integrator_element(
    name = "rk4",
    blurb = "Evaluate evolution with time using Rk4 integration"
)]
struct Rk4 {
    inner: Mutex<InnerRk4>,
}

struct InnerRk4 {
    step: u8,
    original: Vec<Entity>,
    k1: Vec<Entity>,
    k2: Vec<Entity>,
    k3: Vec<Entity>,
    k4: Vec<Entity>,
}

impl IntegratorElement for Rk4 {
    // entity can't be dereferenced
    #[allow(clippy::needless_range_loop)]
    fn integrate(
        &self,
        entities: &[physim_core::Entity],
        new_state: &mut [physim_core::Entity],
        forces: &[physim_core::Force],
        dt: f32,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let k = inner.step.rem(4);
        inner.step += 1;

        match k {
            0 => {
                inner.k1 = vec![Entity::default(); entities.len()];
                inner.k2 = vec![Entity::default(); entities.len()];
                inner.k3 = vec![Entity::default(); entities.len()];
                inner.k4 = vec![Entity::default(); entities.len()];

                inner.original.extend_from_slice(entities);
                for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
                    let m = entity.mass;

                    let a = [f.fx / m, f.fy / m, f.fz / m];

                    let x = entity.x + entity.vx * dt;
                    let y = entity.y + entity.vy * dt;
                    let z = entity.z + entity.vz * dt;

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
                inner.k1 = new_state.to_vec();
            }
            1 => {
                for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
                    let m = entity.mass;
                    let a = [f.fx / m, f.fy / m, f.fz / m];

                    let original = &inner.original[idx];
                    let vx = original.vx + 0.5 * a[0] * dt;
                    let vy = original.vy + 0.5 * a[1] * dt;
                    let vz = original.vz + 0.5 * a[2] * dt;

                    let x = original.x + 0.5 * vx * dt;
                    let y = original.y + 0.5 * vy * dt;
                    let z = original.z + 0.5 * vz * dt;

                    let mut new_entity = *entity;
                    new_entity.x = x;
                    new_entity.y = y;
                    new_entity.z = z;
                    new_entity.vx = vx;
                    new_entity.vy = vy;
                    new_entity.vz = vz;
                    new_state[idx] = new_entity;
                }
                inner.k2 = new_state.to_vec();
            }
            2 => {
                for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
                    let m = entity.mass;
                    let a = [f.fx / m, f.fy / m, f.fz / m];

                    let original = &inner.original[idx];
                    let vx = original.vx + 0.5 * a[0] * dt;
                    let vy = original.vy + 0.5 * a[1] * dt;
                    let vz = original.vz + 0.5 * a[2] * dt;

                    let x = original.x + 0.5 * vx * dt;
                    let y = original.y + 0.5 * vy * dt;
                    let z = original.z + 0.5 * vz * dt;

                    let mut new_entity = *entity;
                    new_entity.x = x;
                    new_entity.y = y;
                    new_entity.z = z;
                    new_entity.vx = vx;
                    new_entity.vy = vy;
                    new_entity.vz = vz;
                    new_state[idx] = new_entity;
                }
                inner.k3 = new_state.to_vec();
            }
            3 => {
                for (idx, (entity, f)) in entities.iter().zip(forces).enumerate() {
                    let m = entity.mass;
                    let a = [f.fx / m, f.fy / m, f.fz / m];

                    let original = &inner.original[idx];
                    let vx = original.vx + a[0] * dt;
                    let vy = original.vy + a[1] * dt;
                    let vz = original.vz + a[2] * dt;

                    let x = original.x + vx * dt;
                    let y = original.y + vy * dt;
                    let z = original.z + vz * dt;

                    let mut new_entity = *entity;
                    new_entity.x = x;
                    new_entity.y = y;
                    new_entity.z = z;
                    new_entity.vx = vx;
                    new_entity.vy = vy;
                    new_entity.vz = vz;
                    inner.k4[idx] = new_entity;
                }

                // Final RK4 combination
                for idx in 0..entities.len() {
                    let o = &inner.original[idx];
                    let k1 = &inner.k1[idx];
                    let k2 = &inner.k2[idx];
                    let k3 = &inner.k3[idx];
                    let k4 = &inner.k4[idx];

                    let mut final_entity = *o;

                    final_entity.x = o.x + (dt / 6.0) * (k1.vx + 2.0 * k2.vx + 2.0 * k3.vx + k4.vx);
                    final_entity.y = o.y + (dt / 6.0) * (k1.vy + 2.0 * k2.vy + 2.0 * k3.vy + k4.vy);
                    final_entity.z = o.z + (dt / 6.0) * (k1.vz + 2.0 * k2.vz + 2.0 * k3.vz + k4.vz);

                    final_entity.vx = o.vx
                        + (dt / 6.0)
                            * ((k1.vx - o.vx)
                                + 2.0 * (k2.vx - o.vx)
                                + 2.0 * (k3.vx - o.vx)
                                + (k4.vx - o.vx));
                    final_entity.vy = o.vy
                        + (dt / 6.0)
                            * ((k1.vy - o.vy)
                                + 2.0 * (k2.vy - o.vy)
                                + 2.0 * (k3.vy - o.vy)
                                + (k4.vy - o.vy));
                    final_entity.vz = o.vz
                        + (dt / 6.0)
                            * ((k1.vz - o.vz)
                                + 2.0 * (k2.vz - o.vz)
                                + 2.0 * (k3.vz - o.vz)
                                + (k4.vz - o.vz));

                    new_state[idx] = final_entity;
                }

                // Reset step counter if needed
                inner.step = 0;
                inner.original.clear();
            }
            _ => unreachable!(),
        }
    }
    fn get_steps(&self) -> usize {
        4
    }
}

impl MessageClient for Rk4 {}

impl ElementCreator for Rk4 {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            inner: Mutex::new(InnerRk4 {
                step: 0,
                original: vec![],
                k1: vec![],
                k2: vec![],
                k3: vec![],
                k4: vec![],
            }),
        })
    }
}

impl Element for Rk4 {
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
