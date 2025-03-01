use crate::{
    render::{Renderable, Vertex},
    Entity,
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

const G: f32 = 1.0; // m3⋅kg−1⋅s−2

#[derive(Clone, Copy, Default, Debug)]
pub struct Star {
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    radius: f32,
    mass: f32,
}

impl Star {
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
        let z: f32 = rng.random_range(-0.45..0.55);

        // let n = (x.powi(2) + y.powi(2)).powf(0.5);
        let vx: f32 = -(y) * 10.0;
        let vy: f32 = x * 10.0;

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

    pub fn suvat(&self, dt: f32, f: [f32; 3]) -> Self {
        let m = self.get_mass();
        // f = ma
        let a = [f[0] / m, f[1] / m, f[2] / m];
        // S = s0 + ut + 1/2 a t^2
        let x = self.x + self.vx * dt + 0.5 * a[0] * (dt.powi(2));
        let y = self.y + self.vy * dt + 0.5 * a[1] * (dt.powi(2));
        let z = self.z + self.vz * dt + 0.5 * a[2] * (dt.powi(2));

        // v = v0 +
        let vx = self.vx + a[0] * dt;
        let vy = self.vy + a[1] * dt;
        let vz = self.vz + a[2] * dt;

        Self {
            x,
            y,
            z,
            vx,
            vy,
            vz,
            mass: self.mass,
            radius: self.radius,
        }
    }

    pub fn newtons_law_of_universal_gravitation(&self, other: &Self) -> [f32; 3] {
        // if Self::inside(&self, other) {
        //     info!("within");
        //     return [0.0,0.0,0.0]
        // }
        let ac = self.get_centre();
        let bc = other.get_centre();

        let r = [bc[0] - ac[0], bc[1] - ac[1], bc[2] - ac[2]];
        let signs = [
            if ac[0] < bc[0] { 1.0 } else { -1.0 },
            if ac[1] < bc[1] { 1.0 } else { -1.0 },
            if ac[2] < bc[2] { 1.0 } else { -1.0 },
        ];

        let am = self.mass;
        let bm = other.mass;
        [
            if r[0].abs() > 1.0 {
                signs[0] * G * am * bm / (r[0].powi(2))
            } else {
                signs[0] * G * am * bm / (1.0_f32.powi(2))
            },
            if r[1].abs() > 1.0 {
                signs[1] * G * am * bm / (r[1].powi(2))
            } else {
                signs[1] * G * am * bm / (1.0_f32.powi(2))
            },
            if r[2].abs() > 1.0 {
                signs[2] * G * am * bm / (r[2].powi(2))
            } else {
                signs[2] * G * am * bm / (1.0_f32.powi(2))
            },
        ]
    }
}

// could implement this so
impl Entity for Star {
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
}

impl Renderable for Star {
    fn verticies(&self) -> Vec<crate::render::Vertex> {
        vec![
            Vertex::new(self.x, self.y + self.radius, self.z),
            Vertex::new(
                self.x - self.radius * f32::sqrt(3.0) * 0.5,
                self.y - 0.5 * self.radius,
                self.z,
            ),
            Vertex::new(
                self.x + self.radius * f32::sqrt(3.0) * 0.5,
                self.y - 0.5 * self.radius,
                self.z,
            ),
        ]
    }
}
