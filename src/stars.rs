use crate::{
    octree::simple::Entity,
    render::{Renderable, Vertex},
};
use rand::Rng;

#[derive(Copy, Clone, Default, Debug)]
pub struct Star {
    x: f32,
    y: f32,
    z: f32,
    radius: f32,
}

impl Star {
    pub fn new(x: f32, y: f32, z: f32, radius: f32) -> Self {
        Self { x, y, z, radius }
    }

    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
            rng.random_range(0.1..0.8),
            rng.random_range(0.001..0.002),
        )
    }

    pub fn update(&self) -> Self {
        let mut rng = rand::rng();
        Self {
            x: self.x + rng.random_range(-0.01..0.01),
            y: self.y + rng.random_range(-0.01..0.01),
            z: self.z + rng.random_range(-0.01..0.01),
            radius: self.radius,
        }
    }
}

impl Entity for Star {
    fn get_mass(&self) -> f32 {
        return 0.02; //self.radius; // obviously nonsense but keep it simple for now.
    }

    fn get_centre(&self) -> [f32; 3] {
        return [self.x, self.y, self.z];
    }

    fn fake(centre: [f32; 3], mass: f32) -> Self {
        Self {
            x: centre[0],
            y: centre[1],
            z: centre[2],
            radius: 0.02_f32.max(mass),
        } // shh I know about r^3
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
