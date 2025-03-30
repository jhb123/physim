use std::collections::HashMap;

use physim_attribute::initialise_state_element;
use physim_core::{
    Entity,
    plugin::initialiser::{InitialStateElement, InitialStateElementCreator},
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
}

impl InitialStateElementCreator for RandomCube {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement> {
        let n = properties
            .get("n")
            .and_then(|v| v.as_u64())
            .unwrap_or(100_000);
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
        let spin = properties.get("s").and_then(|v| v.as_f64()).unwrap_or(0.0);

        Box::new(Self { n, seed, spin })
    }
}

impl InitialStateElement for RandomCube {
    fn create_entities(&self) -> Vec<Entity> {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut state = Vec::with_capacity(self.n as usize);
        for _ in 0..self.n {
            state.push(Entity::random(&mut rng));
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
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "n" => Ok(serde_json::json!(self.n)),
            "seed" => Ok(serde_json::json!(self.seed)),
            "spin" => Ok(serde_json::json!(self.spin)),
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
        ]))
    }
}

#[initialise_state_element(name = "star", blurb = "create a configurable star")]
pub struct SingleStar {
    entity: Entity,
}

impl InitialStateElementCreator for SingleStar {
    fn create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement> {
        fn get_f32(properties: &HashMap<String, Value>, key: &str) -> f32 {
            properties
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or_default()
        }

        let entity = Entity {
            x: get_f32(&properties, "x"),
            y: get_f32(&properties, "y"),
            z: get_f32(&properties, "z"),
            vx: get_f32(&properties, "vx"),
            vy: get_f32(&properties, "vy"),
            vz: get_f32(&properties, "vz"),
            radius: properties
                .get("radius")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(0.1),
            mass: get_f32(&properties, "mass"),
        };

        Box::new(Self { entity })
    }
}

impl InitialStateElement for SingleStar {
    fn create_entities(&self) -> Vec<Entity> {
        vec![self.entity]
    }

    fn set_properties(&mut self, new_props: HashMap<String, Value>) {
        if let Some(val) = new_props.get("x").and_then(|val| val.as_f64()) {
            self.entity.x = val as f32
        }
        if let Some(val) = new_props.get("y").and_then(|val| val.as_f64()) {
            self.entity.y = val as f32
        }
        if let Some(val) = new_props.get("z").and_then(|val| val.as_f64()) {
            self.entity.z = val as f32
        }
        if let Some(val) = new_props.get("vx").and_then(|val| val.as_f64()) {
            self.entity.vx = val as f32
        }
        if let Some(val) = new_props.get("vy").and_then(|val| val.as_f64()) {
            self.entity.vy = val as f32
        }
        if let Some(val) = new_props.get("vz").and_then(|val| val.as_f64()) {
            self.entity.vz = val as f32
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
            "x" => Ok(serde_json::json!(self.entity.x)),
            "y" => Ok(serde_json::json!(self.entity.y)),
            "z" => Ok(serde_json::json!(self.entity.z)),
            "vx" => Ok(serde_json::json!(self.entity.vx)),
            "vy" => Ok(serde_json::json!(self.entity.vy)),
            "vz" => Ok(serde_json::json!(self.entity.vz)),
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
