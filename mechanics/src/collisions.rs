use ahash::RandomState;
use std::collections::HashMap;

use physim_attribute::transmute_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, transmute::TransmuteElement},
};
use serde_json::Value;

#[transmute_element(name = "collisions", blurb = "Add collisions to particles")]
struct Collisions {}

impl TransmuteElement for Collisions {
    fn transmute(&self, data: &mut Vec<Entity>) {
        let cell_size = data
            .iter()
            .map(|e| e.radius)
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.1);
        let grid = Grid::new(data, cell_size);

        for neighbourhood in grid.iter() {
            for i in 0..neighbourhood.len() {
                for j in (i + 1)..neighbourhood.len() {
                    let (ai, bi) = (neighbourhood[i], neighbourhood[j]);

                    let (a, b) = (&data[ai], &data[bi]);

                    let r0 = a.radius;
                    let r1 = b.radius;

                    let dx = a.x - b.x;
                    let dy = a.y - b.y;
                    let dz = a.z - b.z;

                    let dist2 = dx * dx + dy * dy + dz * dz;

                    let min_dist = r0 + r1;

                    if dist2 <= min_dist * min_dist {
                        // Elastic collision response
                        let dvx = a.vx - b.vx;
                        let dvy = a.vy - b.vy;
                        let dvz = a.vz - b.vz;

                        let dot = dvx * dx + dvy * dy + dvz * dz;
                        if dot > 0.0 {
                            continue;
                        } // already separating

                        let ma = a.mass;
                        let mb = b.mass;

                        let scale = 2.0 * dot / ((ma + mb) * dist2);

                        let impulse_a = scale * mb;
                        let impulse_b = scale * ma;

                        data[ai].vx -= impulse_a * dx;
                        data[ai].vy -= impulse_a * dy;
                        data[ai].vz -= impulse_a * dz;

                        data[bi].vx += impulse_b * dx;
                        data[bi].vy += impulse_b * dy;
                        data[bi].vz += impulse_b * dz;
                    }
                }
            }
        }
    }
}

impl MessageClient for Collisions {}

impl ElementCreator for Collisions {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {})
    }
}

impl Element for Collisions {
    fn set_properties(&self, _: HashMap<String, Value>) {}

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err(format!("no property {prop}").into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

type Coordinate = (i32, i32, i32);

#[derive(Debug)]
struct Grid {
    cells: HashMap<Coordinate, Vec<usize>, RandomState>,
}

impl Grid {
    fn new(entities: &[Entity], cell_size: f64) -> Self {
        let mut cells: HashMap<(i32, i32, i32), Vec<usize>, RandomState> = HashMap::default();
        for (i, e) in entities.iter().enumerate() {
            let key = Self::cell_coords(e.x, e.y, e.z, cell_size);
            cells.entry(key).or_default().push(i);
        }
        Self { cells }
    }

    fn cell_coords(x: f64, y: f64, z: f64, cell_size: f64) -> Coordinate {
        let cx = (x / cell_size).floor() as i32;
        let cy = (y / cell_size).floor() as i32;
        let cz = (z / cell_size).floor() as i32;
        (cx, cy, cz)
    }

    fn get_neighbours(coordinate: Coordinate) -> [Coordinate; 27] {
        [
            (coordinate.0 - 1, coordinate.1 - 1, coordinate.2),
            (coordinate.0, coordinate.1 - 1, coordinate.2),
            (coordinate.0 + 1, coordinate.1 - 1, coordinate.2),
            (coordinate.0 - 1, coordinate.1, coordinate.2),
            (coordinate.0, coordinate.1, coordinate.2),
            (coordinate.0 + 1, coordinate.1, coordinate.2),
            (coordinate.0 - 1, coordinate.1 + 1, coordinate.2),
            (coordinate.0, coordinate.1 + 1, coordinate.2),
            (coordinate.0 + 1, coordinate.1 + 1, coordinate.2),
            (coordinate.0 - 1, coordinate.1 - 1, coordinate.2 - 1),
            (coordinate.0, coordinate.1 - 1, coordinate.2 - 1),
            (coordinate.0 + 1, coordinate.1 - 1, coordinate.2 - 1),
            (coordinate.0 - 1, coordinate.1, coordinate.2 - 1),
            (coordinate.0, coordinate.1, coordinate.2 - 1),
            (coordinate.0 + 1, coordinate.1, coordinate.2 - 1),
            (coordinate.0 - 1, coordinate.1 + 1, coordinate.2 - 1),
            (coordinate.0, coordinate.1 + 1, coordinate.2 - 1),
            (coordinate.0 + 1, coordinate.1 + 1, coordinate.2 - 1),
            (coordinate.0 - 1, coordinate.1 - 1, coordinate.2 + 1),
            (coordinate.0, coordinate.1 - 1, coordinate.2 + 1),
            (coordinate.0 + 1, coordinate.1 - 1, coordinate.2 + 1),
            (coordinate.0 - 1, coordinate.1, coordinate.2 + 1),
            (coordinate.0, coordinate.1, coordinate.2 + 1),
            (coordinate.0 + 1, coordinate.1, coordinate.2 + 1),
            (coordinate.0 - 1, coordinate.1 + 1, coordinate.2 + 1),
            (coordinate.0, coordinate.1 + 1, coordinate.2 + 1),
            (coordinate.0 + 1, coordinate.1 + 1, coordinate.2 + 1),
        ]
    }

    fn iter(&self) -> GridIter {
        GridIter {
            grid: self,
            keys: self.cells.keys().cloned().collect(),
            idx: 0,
        }
    }
}

struct GridIter<'a> {
    grid: &'a Grid,
    keys: Vec<Coordinate>,
    idx: usize,
}

impl Iterator for GridIter<'_> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.keys.len() {
            return None;
        }

        let key = self.keys[self.idx];
        self.idx += 1;

        let mut result = Vec::new();

        for neighbor in Grid::get_neighbours(key) {
            if let Some(indices) = self.grid.cells.get(&neighbor) {
                for &i in indices {
                    result.push(i);
                }
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_grid_iter_neighbourhoods() {
        let entities = vec![
            Entity {
                x: 1.0,
                y: 1.0,
                ..Default::default()
            }, // should land in (0,0)
            Entity {
                x: 2.0,
                y: 1.0,
                ..Default::default()
            }, // also (0,0)
            Entity {
                x: 10.0,
                y: 10.0,
                ..Default::default()
            }, // (2,2) if cell_size = 5.0
        ];

        let grid = Grid::new(&entities, 5.0);

        // Collect all neighbourhoods
        let neighbourhoods: Vec<Vec<usize>> = grid.iter().collect();

        // There should be 2 keys -> 2 neighbourhoods
        assert_eq!(neighbourhoods.len(), 2);
        // One of them should contain the first two entities together
        assert!(
            neighbourhoods
                .iter()
                .any(|nh| { nh.contains(&0) && nh.contains(&1) })
        );

        // And the other should contain the third entity
        assert!(neighbourhoods.iter().any(|nh| nh.contains(&2)));
    }
}
