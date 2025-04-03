#![feature(test)]
#![feature(vec_into_raw_parts)]
#![feature(box_as_ptr)]
pub mod pipeline;
pub mod plugin;

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::Serialize;

#[repr(C)]
pub struct UniverseConfiguration {
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
    // edge_mode: UniverseEdge,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
#[repr(C)]
pub struct Entity {
    pub state: EntityState,
    pub prev_state: Option<EntityState>,
    pub radius: f32,
    pub mass: f32,
    pub id: usize,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
#[repr(C)]
pub struct EntityState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}

impl EntityState {
    pub fn random(rng: &mut ChaCha8Rng) -> Self {
        let x: f32 = rng.random_range(-1.0..1.0);
        let y: f32 = rng.random_range(-1.0..1.0);
        let z: f32 = rng.random_range(0.0..1.0);

        Self {
            x,
            y,
            z,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
        }
    }
}

impl Entity {
    pub fn new(x: f32, y: f32, z: f32, mass: f32) -> Self {
        let state = EntityState {
            x,
            y,
            z,
            ..Default::default()
        };

        Self {
            state,
            mass,
            radius: mass.powf(0.33333),
            ..Default::default()
        }
    }
    pub fn new2(x: f32, y: f32, z: f32, mass: f32, radius: f32) -> Self {
        let state = EntityState {
            x,
            y,
            z,
            ..Default::default()
        };

        Self {
            state,
            mass,
            radius,
            ..Default::default()
        }
    }

    pub fn random(rng: &mut ChaCha8Rng) -> Self {
        let state = EntityState::random(rng);

        Self {
            state,
            mass: 0.005,
            radius: 0.02,
            ..Default::default()
        }
    }
}
