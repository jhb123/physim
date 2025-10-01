#![feature(test)]
#![feature(vec_into_raw_parts)]
#![feature(box_as_ptr)]
pub mod messages;
pub mod pipeline;
pub mod plugin;

pub use log;
pub use once_cell;

use std::ops::{Add, AddAssign, Neg, Sub};

use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[repr(C)]
pub struct UniverseConfiguration {
    pub size_x: f64,
    pub size_y: f64,
    pub size_z: f64,
    // edge_mode: UniverseEdge,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Entity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub radius: f64,
    pub mass: f64,
    pub id: usize,
    pub fixed: bool,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
pub struct Acceleration {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Acceleration {
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Add for Acceleration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Acceleration {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Acceleration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Neg for Acceleration {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Entity {
    pub fn new(x: f64, y: f64, z: f64, mass: f64) -> Self {
        Self {
            x,
            y,
            z,
            mass,
            radius: mass.powf(0.33333),
            ..Default::default()
        }
    }
    pub fn new2(x: f64, y: f64, z: f64, mass: f64, radius: f64) -> Self {
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
        let x: f64 = rng.random_range(-1.0..1.0);
        let y: f64 = rng.random_range(-1.0..1.0);
        let z: f64 = rng.random_range(0.0..1.0);

        Self {
            x,
            y,
            z,
            mass: 0.005,
            radius: 0.02,
            ..Default::default()
        }
    }
}
