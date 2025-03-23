use std::collections::HashMap;

use physim_attribute::initialise_state_element;
use physim_core::{
    ElementInfo, ElementKind, Entity, InitialStateElement, InitialStateElementCreator,
};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use serde_json::Value;

#[initialise_state_element("cube")]
pub struct RandomCube {
    n: u64,
    seed: u64,
}

impl InitialStateElementCreator for RandomCube {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement> {
        let n = properties
            .get("n")
            .and_then(|v| v.as_u64())
            .unwrap_or(100_000);
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);

        Box::new(Self { n, seed })
    }
}

impl InitialStateElement for RandomCube {
    fn initialise(&self) -> Vec<Entity> {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut state = Vec::with_capacity(self.n as usize);
        for _ in 0..self.n {
            state.push(Entity::random(&mut rng));
        }
        state
    }
}

#[initialise_state_element("star")]
pub struct SingleStar {
    entity: Entity,
}

impl InitialStateElementCreator for SingleStar {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement> {
        fn get_f32(properties: &HashMap<String, Value>, key: &str) -> f32 {
            properties
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or_default()
        }

        let entity = Entity {
            x: get_f32(&properties, "x"),
            y: get_f32(&properties, "y"),
            z: get_f32(&properties, "z"),
            vx: get_f32(&properties, "vx"),
            vy: get_f32(&properties, "vy"),
            vz: get_f32(&properties, "vz"),
            radius: properties
                .get("radius")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(0.1),
            mass: get_f32(&properties, "mass"),
        };

        Box::new(Self { entity })
    }
}

impl InitialStateElement for SingleStar {
    fn initialise(&self) -> Vec<Entity> {
        vec![self.entity]
    }
}
