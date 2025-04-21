use rand::Rng;
use std::{collections::HashMap, f32::consts::PI};

use physim_attribute::initialise_state_element;
use physim_core::{
    Entity, EntityState,
    plugin::generator::{GeneratorElement, GeneratorElementCreator},
};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use serde_json::Value;

#[initialise_state_element(
    name = "cube",
    blurb = "Generate a galaxy where stars are randomly placed in a cubic volume."
)]
#[derive(Debug)]
pub struct RandomCube {
    n: u64,
    seed: u64,
    spin: f64,
    centre: [f32; 3],
    size: f32,
}

impl GeneratorElementCreator for RandomCube {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn GeneratorElement> {
        let n = properties
            .get("n")
            .and_then(|v| v.as_u64())
            .unwrap_or(100_000);
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
        let spin = properties
            .get("spin")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let size = properties
            .get("size")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let centre = properties
            .get("centre")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f32> = coords
                        .iter()
                        .flat_map(|x| x.as_f64())
                        .map(|x| x as f32)
                        .collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f32; 3]);
        Box::new(Self {
            n,
            seed,
            spin,
            centre,
            size,
        })
    }
}

impl GeneratorElement for RandomCube {
    fn create_entities(&self) -> Vec<Entity> {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut state = Vec::with_capacity(self.n as usize);
        for _ in 0..self.n {
            let mut e = Entity::random(&mut rng);
            e.state.x *= self.size;
            e.state.y *= self.size;
            e.state.z *= self.size;

            e.state.vx = e.state.y * self.spin as f32;
            e.state.vy = -e.state.x * self.spin as f32;

            e.state.x += self.centre[0];
            e.state.y += self.centre[1];
            e.state.z += self.centre[2];

            state.push(e);
        }
        state
    }

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(n) = new_props.get("n").and_then(|n| n.as_u64()) {
            self.n = n
        }
        if let Some(s) = new_props.get("s").and_then(|s| s.as_f64()) {
            self.spin = s
        }
        if let Some(seed) = new_props.get("seed").and_then(|seed| seed.as_u64()) {
            self.seed = seed
        }
        if let Some(size) = new_props.get("size").and_then(|size| size.as_f64()) {
            self.size = size as f32
        }
        if let Some(centre) = new_props.get("centre").and_then(|v| {
            let coords = v.as_array()?;
            if coords.len() != 3 {
                None
            } else {
                let coords: Vec<f32> = coords
                    .iter()
                    .flat_map(|x| x.as_f64())
                    .map(|x| x as f32)
                    .collect();
                Some([coords[0], coords[1], coords[2]])
            }
        }) {
            self.centre = centre
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "n" => Ok(serde_json::json!(self.n)),
            "seed" => Ok(serde_json::json!(self.seed)),
            "spin" => Ok(serde_json::json!(self.spin)),
            "size" => Ok(serde_json::json!(self.spin)),
            "centre" => Ok(serde_json::json!(self.spin)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            ("n".to_string(), "Number of stars".to_string()),
            ("seed".to_string(), "Random seed".to_string()),
            ("spin".to_string(), "Spin factor v = (r*s)".to_string()),
            ("size".to_string(), "side length of cube".to_string()),
            (
                "centre".to_string(),
                "Centre (specify in CLI with \\[x,y,z\\])".to_string(),
            ),
        ]))
    }
}

#[initialise_state_element(name = "star", blurb = "create a configurable star")]
pub struct SingleStar {
    entity: Entity,
}

impl GeneratorElementCreator for SingleStar {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn GeneratorElement> {
        fn get_f32(properties: &HashMap<String, Value>, key: &str) -> f32 {
            properties
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or_default()
        }

        let entity = Entity {
            state: EntityState {
                x: get_f32(&properties, "x"),
                y: get_f32(&properties, "y"),
                z: get_f32(&properties, "z"),
                vx: get_f32(&properties, "vx"),
                vy: get_f32(&properties, "vy"),
                vz: get_f32(&properties, "vz"),
            },
            radius: properties
                .get("radius")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(0.1),
            mass: get_f32(&properties, "mass"),
            ..Default::default()
        };

        Box::new(Self { entity })
    }
}

impl GeneratorElement for SingleStar {
    fn create_entities(&self) -> Vec<Entity> {
        vec![self.entity]
    }

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(val) = new_props.get("x").and_then(|val| val.as_f64()) {
            self.entity.state.x = val as f32
        }
        if let Some(val) = new_props.get("y").and_then(|val| val.as_f64()) {
            self.entity.state.y = val as f32
        }
        if let Some(val) = new_props.get("z").and_then(|val| val.as_f64()) {
            self.entity.state.z = val as f32
        }
        if let Some(val) = new_props.get("vx").and_then(|val| val.as_f64()) {
            self.entity.state.vx = val as f32
        }
        if let Some(val) = new_props.get("vy").and_then(|val| val.as_f64()) {
            self.entity.state.vy = val as f32
        }
        if let Some(val) = new_props.get("vz").and_then(|val| val.as_f64()) {
            self.entity.state.vz = val as f32
        }
        if let Some(val) = new_props.get("m").and_then(|val| val.as_f64()) {
            self.entity.mass = val as f32
        }
        if let Some(val) = new_props.get("r").and_then(|val| val.as_f64()) {
            self.entity.radius = val as f32
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "x" => Ok(serde_json::json!(self.entity.state.x)),
            "y" => Ok(serde_json::json!(self.entity.state.y)),
            "z" => Ok(serde_json::json!(self.entity.state.z)),
            "vx" => Ok(serde_json::json!(self.entity.state.vx)),
            "vy" => Ok(serde_json::json!(self.entity.state.vy)),
            "vz" => Ok(serde_json::json!(self.entity.state.vz)),
            "m" => Ok(serde_json::json!(self.entity.mass)),
            "r" => Ok(serde_json::json!(self.entity.radius)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            ("x".to_string(), "x position".to_string()),
            ("y".to_string(), "y position".to_string()),
            ("z".to_string(), "z position".to_string()),
            ("vx".to_string(), "velocity in x direction".to_string()),
            ("vy".to_string(), "velocity in y direction".to_string()),
            ("vz".to_string(), "velocity in z direction".to_string()),
            ("m".to_string(), "mass".to_string()),
            ("r".to_string(), "Radius (screen units)".to_string()),
        ]))
    }
}

#[initialise_state_element(
    name = "plummer",
    blurb = "Generate a galaxy where stars are distributed using a Plummer model."
)]
#[derive(Debug)]
pub struct Plummer {
    n: u64,
    seed: u64,
    mass: f64,
    centre: [f32; 3],
    initial_v: [f32; 3],
    plummer_r: f32,
    spin: f32,
}

impl GeneratorElementCreator for Plummer {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn GeneratorElement> {
        let n = properties
            .get("n")
            .and_then(|v| v.as_u64())
            .unwrap_or(100_000);
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
        let mass = properties
            .get("mass")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let plummer_r = properties.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let centre = properties
            .get("centre")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f32> = coords
                        .iter()
                        .flat_map(|x| x.as_f64())
                        .map(|x| x as f32)
                        .collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f32; 3]);

        let initial_v = properties
            .get("v")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f32> = coords
                        .iter()
                        .flat_map(|x| x.as_f64())
                        .map(|x| x as f32)
                        .collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f32; 3]);

        let spin = properties
            .get("spin")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Box::new(Self {
            n,
            seed,
            mass,
            centre,
            initial_v,
            plummer_r,
            spin,
        })
    }
}

impl GeneratorElement for Plummer {
    fn create_entities(&self) -> Vec<Entity> {
        let rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut state = Vec::with_capacity(self.n as usize);

        let cdf: Vec<f32> = rng.random_iter().take((self.n * 3) as usize).collect();
        let m = (self.mass as f32) / (self.n as f32);
        for c in cdf.chunks(3) {
            let r = self.plummer_r * (c[0].powf(-2_f32 / 3_f32) - 1_f32).powf(0.5);
            let theta = PI * c[2];
            let phi = 2_f32 * PI * c[1];

            let x = r * theta.sin() * phi.cos() + self.centre[0];
            let y = r * theta.sin() * phi.sin() + self.centre[1];
            let z = r * theta.cos() + self.centre[2];
            let mut e = Entity::new2(x, y, z, m, 0.01);

            let r2 = r.powi(2);
            let a2 = self.plummer_r.powi(2);

            let v_phi = self.spin * ((self.mass as f32) * r2 / (r2 + a2).powf(1.5)).sqrt();

            let vx = -v_phi * phi.sin();
            let vy = v_phi * phi.cos();
            let vz = 0.0;

            e.state.vx = vx + self.initial_v[0];
            e.state.vy = vy + self.initial_v[1];
            e.state.vz = vz + self.initial_v[2];
            state.push(e);
        }
        state
    }

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(n) = new_props.get("n").and_then(|n| n.as_u64()) {
            self.n = n
        }
        if let Some(m) = new_props.get("mass").and_then(|s| s.as_f64()) {
            self.mass = m
        }
        if let Some(seed) = new_props.get("seed").and_then(|seed| seed.as_u64()) {
            self.seed = seed
        }
        if let Some(a) = new_props.get("a").and_then(|a| a.as_f64()) {
            self.plummer_r = a as f32
        }
        if let Some(centre) = new_props.get("centre").and_then(|v| {
            let coords = v.as_array()?;
            if coords.len() != 3 {
                None
            } else {
                let coords: Vec<f32> = coords
                    .iter()
                    .flat_map(|x| x.as_f64())
                    .map(|x| x as f32)
                    .collect();
                Some([coords[0], coords[1], coords[2]])
            }
        }) {
            self.centre = centre
        }
        if let Some(initial_v) = new_props.get("v").and_then(|v| {
            let coords = v.as_array()?;
            if coords.len() != 3 {
                None
            } else {
                let coords: Vec<f32> = coords
                    .iter()
                    .flat_map(|x| x.as_f64())
                    .map(|x| x as f32)
                    .collect();
                Some([coords[0], coords[1], coords[2]])
            }
        }) {
            self.initial_v = initial_v
        }
        if let Some(spin) = new_props.get("spin").and_then(|spin| spin.as_f64()) {
            self.spin = spin as f32;
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "n" => Ok(serde_json::json!(self.n)),
            "seed" => Ok(serde_json::json!(self.seed)),
            "spin" => Ok(serde_json::json!(self.spin)),
            "a" => Ok(serde_json::json!(self.plummer_r)),
            "mass" => Ok(serde_json::json!(self.mass)),
            "centre" => Ok(serde_json::json!(self.centre)),
            "v" => Ok(serde_json::json!(self.initial_v)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            ("n".to_string(), "Number of stars".to_string()),
            ("seed".to_string(), "Random seed".to_string()),
            ("mass".to_string(), "Mass of Galaxy".to_string()),
            ("spin".to_string(), "Spin factor".to_string()),
            ("a".to_string(), "Plummer radius".to_string()),
            (
                "centre".to_string(),
                "Centre (specify in CLI with \\[x,y,z\\])".to_string(),
            ),
            (
                "v".to_string(),
                "velocity (specify in CLI with \\[vx,vy,vz\\])".to_string(),
            ),
        ]))
    }
}
