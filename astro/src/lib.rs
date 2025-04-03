#![feature(test)]
#![feature(str_from_raw_parts)]
#![feature(vec_into_raw_parts)]
mod initialisers;
mod octree;
mod quadtree;
mod transformers;

use std::ffi::CString;

use physim_core::{Entity, EntityState, register_plugin};

// static ELEMENTS: &str = "astro,simple_astro,debug";
register_plugin!("astro", "astro2", "simple_astro", "debug", "cube", "star");

const G: f32 = 1.0;

pub trait Star {
    fn get_mass(&self) -> f32;
    fn get_centre(&self) -> [f32; 3];
    fn centre_of_mass(&self, other: &Self) -> [f32; 3];
    fn fake(centre: [f32; 3], mass: f32) -> Self;
    fn inside(a: &Self, b: &Self) -> bool;
    fn newtons_law_of_universal_gravitation(&self, other: &Self, easing_factor: f32) -> [f32; 3];
    fn euler(&self, dt: f32, f: [f32; 3]) -> Self;
    fn verlet(&self, dt: f32, f: [f32; 3]) -> Self;
}

// could implement this so
impl Star for Entity {
    fn centre_of_mass(&self, other: &Self) -> [f32; 3] {
        let total_mass = self.mass + other.mass;

        let inv_total_mass = 1.0 / total_mass;

        [
            (self.mass * self.state.x + other.mass * other.state.x) * inv_total_mass,
            (self.mass * self.state.y + other.mass * other.state.y) * inv_total_mass,
            (self.mass * self.state.z + other.mass * other.state.z) * inv_total_mass,
        ]
    }

    fn get_mass(&self) -> f32 {
        self.mass // this is not real physics.
    }

    fn get_centre(&self) -> [f32; 3] {
        // assert!(self.state.x.is_normal()) ;
        // assert!(self.state.y.is_normal()) ;
        // assert!(self.state.z.is_normal()) ;

        [self.state.x, self.state.y, self.state.z]
    }

    fn fake(centre: [f32; 3], mass: f32) -> Self {
        if centre[0].is_nan() {
            panic!()
        }
        Self {
            state: EntityState {
                x: centre[0],
                y: centre[1],
                z: centre[2],
                ..Default::default()
            },
            radius: 0.0, // hm
            mass,
            ..Default::default()
        }
    }

    fn inside(a: &Self, b: &Self) -> bool {
        ((a.state.x - b.state.x).abs() < a.radius / 2.0
            || (a.state.x - b.state.x).abs() < b.radius / 2.0)
            && ((a.state.y - b.state.y).abs() < a.radius / 2.0
                || (a.state.y - b.state.y).abs() < b.radius / 2.0)
            && ((a.state.z - b.state.z).abs() < a.radius / 2.0
                || (a.state.z - b.state.z).abs() < b.radius / 2.0)
    }

    fn newtons_law_of_universal_gravitation(&self, other: &Self, easing_factor: f32) -> [f32; 3] {
        // if Self::inside(&self, other) {
        //     info!("within");
        //     return [0.0,0.0,0.0]
        // }
        let ac = self.get_centre();
        let bc = other.get_centre();

        let r_norm =
            ((ac[0] - bc[0]).powi(2) + (ac[1] - bc[1]).powi(2) + (ac[2] - bc[2]).powi(2)).powf(0.5);

        let r_easing = (ac[0] - bc[0]).powi(2)
            + (ac[1] - bc[1]).powi(2)
            + (ac[2] - bc[2]).powi(2)
            + easing_factor;

        let r = [
            (bc[0] - ac[0]) / r_norm,
            (bc[1] - ac[1]) / r_norm,
            (bc[2] - ac[2]) / r_norm,
        ];

        let am = self.mass;
        let bm = other.mass;
        [
            r[0] * G * am * bm / r_easing,
            r[1] * G * am * bm / r_easing,
            r[2] * G * am * bm / r_easing,
        ]
    }

    fn euler(&self, dt: f32, f: [f32; 3]) -> Self {
        let m = self.get_mass();
        // f = ma
        let a = [f[0] / m, f[1] / m, f[2] / m];
        // S = s0 + ut + 1/2 a t^2
        let x = self.state.x + self.state.vx * dt + 0.5 * a[0] * (dt.powi(2));
        let y = self.state.y + self.state.vy * dt + 0.5 * a[1] * (dt.powi(2));
        let z = self.state.z + self.state.vz * dt + 0.5 * a[2] * (dt.powi(2));

        // v = v0 +
        let vx = self.state.vx + a[0] * dt;
        let vy = self.state.vy + a[1] * dt;
        let vz = self.state.vz + a[2] * dt;

        Self {
            state: EntityState {
                x,
                y,
                z,
                vx,
                vy,
                vz,
            },
            mass: self.mass,
            radius: self.radius,
            ..Default::default()
        }
    }

    fn verlet(&self, dt: f32, f: [f32; 3]) -> Self {
        let m = self.get_mass();
        // f = ma
        let a = [f[0] / m, f[1] / m, f[2] / m];
        // S = s0 + ut + 1/2 a t^2

        let (x, y, z) = match self.prev_state {
            Some(prev_state) => {
                let x = 2_f32 * self.state.x - prev_state.x + a[0] * (dt.powi(2));
                let y = 2_f32 * self.state.y - prev_state.y + a[1] * (dt.powi(2));
                let z = 2_f32 * self.state.z - prev_state.z + a[2] * (dt.powi(2));
                (x, y, z)
            }
            None => {
                let x = self.state.x + self.state.vx * dt + 0.5 * a[0] * (dt.powi(2));
                let y = self.state.y + self.state.vy * dt + 0.5 * a[1] * (dt.powi(2));
                let z = self.state.z + self.state.vz * dt + 0.5 * a[2] * (dt.powi(2));
                (x, y, z)
            }
        };

        // v = v0 +
        let vx = (x - self.state.x) / dt;
        let vy = (y - self.state.y) / dt;
        let vz = (z - self.state.z) / dt;

        Self {
            state: EntityState {
                x,
                y,
                z,
                vx,
                vy,
                vz,
            },
            prev_state: Some(self.state),
            mass: self.mass,
            radius: self.radius,
            ..Default::default()
        }
    }
}
