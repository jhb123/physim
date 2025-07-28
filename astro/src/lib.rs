#![feature(test)]
#![feature(str_from_raw_parts)]
#![feature(vec_into_raw_parts)]
#![feature(trait_alias)]

mod initialisers;
pub mod octree;
pub mod quadtree;
mod transformers;

use physim_core::{Entity, register_plugin};

// static ELEMENTS: &str = "astro,simple_astro,debug";
register_plugin!("astro", "astro2", "simple_astro", "cube", "star", "plummer");

// make a function that when is called, sets a global bus variable in dynamic library

const G: f32 = 1.0;

pub trait Star {
    fn get_mass(&self) -> f32;
    fn get_centre(&self) -> [f32; 3];
    fn centre_of_mass(&self, other: &Self) -> [f32; 3];
    fn fake(centre: [f32; 3], mass: f32) -> Self;
    fn inside(a: &Self, b: &Self) -> bool;
    fn newtons_law_of_universal_gravitation(&self, other: &Self, easing_factor: f32) -> [f32; 3];
}

// could implement this so
impl Star for Entity {
    fn centre_of_mass(&self, other: &Self) -> [f32; 3] {
        let total_mass = self.mass + other.mass;

        let inv_total_mass = 1.0 / total_mass;

        [
            (self.mass * self.x + other.mass * other.x) * inv_total_mass,
            (self.mass * self.y + other.mass * other.y) * inv_total_mass,
            (self.mass * self.z + other.mass * other.z) * inv_total_mass,
        ]
    }

    fn get_mass(&self) -> f32 {
        self.mass // this is not real physics.
    }

    fn get_centre(&self) -> [f32; 3] {
        // assert!(self.x.is_normal()) ;
        // assert!(self.y.is_normal()) ;
        // assert!(self.z.is_normal()) ;

        [self.x, self.y, self.z]
    }

    fn fake(centre: [f32; 3], mass: f32) -> Self {
        if centre[0].is_nan() {
            panic!()
        }
        Self {
            x: centre[0],
            y: centre[1],
            z: centre[2],
            radius: 0.0, // hm
            mass,
            ..Default::default()
        }
    }

    fn inside(a: &Self, b: &Self) -> bool {
        ((a.x - b.x).abs() < a.radius / 2.0 || (a.x - b.x).abs() < b.radius / 2.0)
            && ((a.y - b.y).abs() < a.radius / 2.0 || (a.y - b.y).abs() < b.radius / 2.0)
            && ((a.z - b.z).abs() < a.radius / 2.0 || (a.z - b.z).abs() < b.radius / 2.0)
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
}
