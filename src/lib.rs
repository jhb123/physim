#[macro_use]
extern crate glium;

pub mod octree;
pub mod quadtree;
pub mod render;
pub mod stars;

pub struct UniverseConfiguration {
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
    // edge_mode: UniverseEdge,
}

pub trait Entity {
    fn get_mass(&self) -> f32;
    fn get_centre(&self) -> [f32; 3];
    fn fake(centre: [f32; 3], mass: f32) -> Self;
}
