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
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub radius: f32,
    pub mass: f32,
}

impl Entity {
    pub fn new(x: f32, y: f32, z: f32, mass: f32) -> Self {
        Self {
            x,
            y,
            z,
            mass,
            radius: mass.powf(0.33333),
            ..Default::default()
        }
    }
    pub fn new2(x: f32, y: f32, z: f32, mass: f32, radius: f32) -> Self {
        Self {
            x,
            y,
            z,
            mass,
            radius,
            ..Default::default()
        }
    }

    pub fn random(rng: &mut ChaCha8Rng) -> Self {
        let x: f32 = rng.random_range(-1.0..1.0);
        let y: f32 = rng.random_range(-1.0..1.0);
        let z: f32 = rng.random_range(0.0..1.0);

        // let n = (x.powi(2) + y.powi(2)).powf(0.5);
        let vx: f32 = -(y) * 300.0;
        let vy: f32 = x * 300.0;

        // let vy: f32 = 0.05*(x)/n;

        Self {
            x,
            y,
            z,
            vx,
            vy,
            vz: 0.0,
            mass: 0.005,
            radius: 0.02,
        }
    }

    pub fn jitter_position(&self) -> Self {
        let mut rng = rand::rng();
        Self {
            x: self.x + rng.random_range(-0.01..0.01),
            y: self.y + rng.random_range(-0.01..0.01),
            z: self.z + rng.random_range(-0.01..0.01),
            radius: self.radius,
            ..Default::default()
        }
    }

    pub fn jitter_position_inplace(&mut self) {
        let mut rng = rand::rng();
        self.x += rng.random_range(-0.01..0.01);
        self.y += rng.random_range(-0.01..0.01);
        self.z += rng.random_range(-0.01..0.01);
    }
}
