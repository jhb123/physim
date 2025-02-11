#[macro_use]
extern crate glium;

pub mod render;
pub mod stars;

enum UniverseEdge {
    Infinite,
    WrapAround,
}
pub struct UniverseConfiguration {
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
    // edge_mode: UniverseEdge,
}

// trait Entity {
//     fn new_normalised(config: UniverseConfiguration, ... ) -> Self {
//         f = config.factors;
//         Self { p1/f.1 ... }
//     }
// }