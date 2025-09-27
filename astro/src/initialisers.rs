use core::f64;
use rand::Rng;
use rand_distr::Distribution;
use serde::Serialize;
use std::{collections::HashMap, f64::consts::PI, sync::Mutex};

use physim_attribute::initialise_state_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, generator::GeneratorElement},
};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use serde_json::Value;

#[initialise_state_element(
    name = "cube",
    blurb = "Generate a galaxy where stars are randomly placed in a cubic volume."
)]
#[derive(Debug)]
pub struct RandomCube {
    inner: Mutex<InnerRandomCube>,
}

#[derive(Debug, Serialize)]
struct InnerRandomCube {
    n: u64,
    seed: u64,
    spin: f64,
    centre: [f64; 3],
    size: f64,
    mass: f64,
    id: usize,
}

impl ElementCreator for RandomCube {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
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
            .unwrap_or(1.0);
        let centre = properties
            .get("centre")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f64> = coords.iter().flat_map(|x| x.as_f64()).collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f64; 3]);
        let mass = properties
            .get("mass")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let id = properties.get("id").and_then(|id| id.as_u64()).unwrap_or(0);
        Box::new(Self {
            inner: Mutex::new(InnerRandomCube {
                n,
                seed,
                spin,
                centre,
                size,
                mass,
                id: id as usize,
            }),
        })
    }
}

impl GeneratorElement for RandomCube {
    fn create_entities(&self) -> Vec<Entity> {
        let element = self.inner.lock().unwrap();

        let mut rng = ChaCha8Rng::seed_from_u64(element.seed);
        let entity_mass = element.mass / (element.n as f64);
        let mut state = Vec::with_capacity(element.n as usize);
        for _ in 0..element.n {
            let mut e = Entity::random(&mut rng);
            e.x *= element.size;
            e.y *= element.size;
            e.z *= element.size;

            e.vx = e.y * element.spin;
            e.vy = -e.x * element.spin;

            e.x += element.centre[0];
            e.y += element.centre[1];
            e.z += element.centre[2];

            e.mass = entity_mass;
            e.id = element.id;
            state.push(e);
        }
        state
    }
}
impl Element for RandomCube {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            ("n".to_string(), "Number of stars".to_string()),
            ("seed".to_string(), "Random seed".to_string()),
            ("spin".to_string(), "Spin factor v = (r*s)".to_string()),
            ("size".to_string(), "side length of cube".to_string()),
            (
                "mass".to_string(),
                "Total mass of cube. Default is 1.0".to_string(),
            ),
            (
                "centre".to_string(),
                "Centre (specify in CLI with \\[x,y,z\\])".to_string(),
            ),
            (
                "id".to_string(),
                "Id assigned to stars in galaxy".to_string(),
            ),
        ]))
    }
}

impl Serialize for RandomCube {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.lock().unwrap().serialize(serializer)
    }
}

impl MessageClient for RandomCube {}

#[initialise_state_element(name = "star", blurb = "create a configurable star")]
pub struct SingleStar {
    inner: Mutex<SingleStarInner>,
}

struct SingleStarInner {
    entity: Entity,
}

impl ElementCreator for SingleStar {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        fn get_f64(properties: &HashMap<String, Value>, key: &str) -> f64 {
            properties
                .get(key)
                .and_then(|v| v.as_f64())
                .unwrap_or_default()
        }

        let entity = Entity {
            x: get_f64(&properties, "x"),
            y: get_f64(&properties, "y"),
            z: get_f64(&properties, "z"),
            vx: get_f64(&properties, "vx"),
            vy: get_f64(&properties, "vy"),
            vz: get_f64(&properties, "vz"),
            radius: properties
                .get("radius")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1),
            mass: get_f64(&properties, "mass"),
            id: properties
                .get("id")
                .and_then(|v| v.as_u64().map(|v| v as usize))
                .unwrap_or(0),
            fixed: properties
                .get("fixed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        let inner = SingleStarInner { entity };

        Box::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

impl MessageClient for SingleStar {}

impl GeneratorElement for SingleStar {
    fn create_entities(&self) -> Vec<Entity> {
        let inner = self.inner.lock().unwrap();
        vec![inner.entity]
    }
}

impl Element for SingleStar {
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
            ("id".to_string(), "ID of entity".to_string()),
            ("fixed".to_string(), "Fix location".to_string()),
        ]))
    }
}

#[initialise_state_element(
    name = "plummer",
    blurb = "Generate a galaxy where stars are distributed using a Plummer model."
)]
#[derive(Debug)]
pub struct Plummer {
    inner: Mutex<InnerPlummer>,
}

#[derive(Debug)]
struct InnerPlummer {
    n: u64,
    seed: u64,
    mass: f64,
    centre: [f64; 3],
    initial_v: [f64; 3],
    plummer_r: f64,
    spin: f64,
    id: usize,
}

impl ElementCreator for Plummer {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        let n = properties
            .get("n")
            .and_then(|v| v.as_u64())
            .unwrap_or(100_000);
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
        let mass = properties
            .get("mass")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let plummer_r = properties.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let centre = properties
            .get("centre")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f64> = coords.iter().flat_map(|x| x.as_f64()).collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f64; 3]);

        let initial_v = properties
            .get("v")
            .and_then(|v| {
                let coords = v.as_array()?;
                if coords.len() != 3 {
                    None
                } else {
                    let coords: Vec<f64> = coords.iter().flat_map(|x| x.as_f64()).collect();
                    Some([coords[0], coords[1], coords[2]])
                }
            })
            .unwrap_or([0.0_f64; 3]);

        let spin = properties
            .get("spin")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let id = properties.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

        Box::new(Self {
            inner: Mutex::new(InnerPlummer {
                n,
                seed,
                mass,
                centre,
                initial_v,
                plummer_r,
                spin,
                id: id as usize,
            }),
        })
    }
}

impl GeneratorElement for Plummer {
    fn create_entities(&self) -> Vec<Entity> {
        let element = self.inner.lock().unwrap();
        let rng = ChaCha8Rng::seed_from_u64(element.seed);
        let mut state = Vec::with_capacity(element.n as usize);

        let cdf: Vec<f64> = rng.random_iter().take((element.n * 3) as usize).collect();
        let m = (element.mass) / (element.n as f64);
        for c in cdf.chunks(3) {
            let r = element.plummer_r * (c[0].powf(-2_f64 / 3_f64) - 1_f64).powf(0.5);
            let theta = PI * c[2];
            let phi = 2_f64 * PI * c[1];

            let x = r * theta.sin() * phi.cos() + element.centre[0];
            let y = r * theta.sin() * phi.sin() + element.centre[1];
            let z = r * theta.cos() + element.centre[2];
            let mut e = Entity::new2(x, y, z, m, 0.01);

            let r2 = r.powi(2);
            let a2 = element.plummer_r.powi(2);

            let v_phi = element.spin * ((element.mass) * r2 / (r2 + a2).powf(1.5)).sqrt();

            let vx = -v_phi * phi.sin();
            let vy = v_phi * phi.cos();
            let vz = 0.0;

            e.vx = vx + element.initial_v[0];
            e.vy = vy + element.initial_v[1];
            e.vz = vz + element.initial_v[2];

            e.id = element.id;

            state.push(e);
        }
        state
    }
}

impl Element for Plummer {
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
            (
                "id".to_string(),
                "Id assigned to stars in galaxy".to_string(),
            ),
        ]))
    }
}

impl MessageClient for Plummer {}

#[initialise_state_element(name = "solar", blurb = "Generate a toy solar system")]
pub struct SolarSystem {
    seed: u64,
    planets: u64,
    asteroids: u64,
}

impl ElementCreator for SolarSystem {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        let seed = properties.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
        let planets = properties
            .get("planets")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let asteroids = properties
            .get("asteroids")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Box::new(Self {
            seed,
            planets,
            asteroids,
        })
    }
}

impl GeneratorElement for SolarSystem {
    // https://arxiv.org/pdf/1803.00777
    // https://arxiv.org/abs/1502.05011
    fn create_entities(&self) -> Vec<Entity> {
        let rng = ChaCha8Rng::seed_from_u64(self.seed);

        let sun = Entity {
            x: 0.0,
            y: 0.0,
            z: 0.5,
            radius: 0.1,
            mass: 1.0,
            fixed: true,
            ..Default::default()
        };
        let mut entities = vec![sun];

        let m_planets = rand_distr::LogNormal::new(1.1, 0.1)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.planets as usize);
        let r_planets = rand_distr::LogNormal::new(0.2, 0.7)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.planets as usize);
        let angle = rand_distr::Uniform::new(0.0, 2.0 * f64::consts::PI)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.planets as usize);

        let planets: Vec<Entity> = r_planets
            .zip(m_planets)
            .zip(angle)
            .map(|((r, m), theta)| {
                let x = r * theta.sin();
                let y = r * theta.cos();
                let vx = -theta.cos() * 1.0 / r.powf(0.5);
                let vy = theta.sin() * 1.0 / r.powf(0.5);
                Entity {
                    x,
                    y,
                    vx,
                    vy,
                    z: 0.5,
                    radius: 0.05,
                    mass: m * 1e-5,
                    ..Default::default()
                }
            })
            .collect();

        let moons: Vec<Entity> = planets
            .iter()
            .cloned()
            .map(|mut e| {
                e.mass /= 10.0;
                e.x += 0.01;
                e.vy += (e.mass / 0.01).powf(0.5);
                e.radius = 0.005;
                e
            })
            .collect();

        let r_asteroids = rand_distr::LogNormal::new(0.0, 0.9)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.asteroids as usize);
        let angle = rand_distr::Uniform::new(0.0, 2.0 * f64::consts::PI)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.asteroids as usize);
        let eccentricities = rand_distr::Uniform::new(0.0, 0.6)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.asteroids as usize);

        let orientations = rand_distr::Uniform::new(0.0, 2.0 * f64::consts::PI)
            .unwrap()
            .sample_iter(rng.clone())
            .take(self.asteroids as usize);

        let asteroids: Vec<Entity> = r_asteroids
            .zip(angle)
            .zip(eccentricities)
            .zip(orientations)
            .map(|(((a, theta), e), phi)| {
                // Ellipse parameters
                let b = a * (1.0_f64 - e * e).sqrt();

                // Position in unrotated ellipse coords
                let x0 = a * theta.cos();
                let y0 = b * theta.sin();

                // Current distance from focus
                let r_current = (x0 * x0 + y0 * y0).sqrt();
                let mu = 1.0; // gravitational parameter
                let v_mag = (mu * (2.0 / r_current - 1.0 / a)).sqrt();

                // Tangent direction in unrotated coords
                let dx_dt = -a * theta.sin();
                let dy_dt = b * theta.cos();
                let norm = (dx_dt * dx_dt + dy_dt * dy_dt).sqrt();
                let vx0 = v_mag * dx_dt / norm;
                let vy0 = v_mag * dy_dt / norm;

                // Rotate position and velocity by random orientation phi
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();
                let x = x0 * cos_phi - y0 * sin_phi;
                let y = x0 * sin_phi + y0 * cos_phi;
                let vx = vx0 * cos_phi - vy0 * sin_phi;
                let vy = vx0 * sin_phi + vy0 * cos_phi;

                Entity {
                    x,
                    y,
                    vx,
                    vy,
                    z: 0.5,
                    radius: 0.01,
                    mass: 1e-7,
                    ..Default::default()
                }
            })
            .collect();

        entities.extend(planets);
        entities.extend(moons);
        entities.extend(asteroids);
        entities
    }
}

impl Element for SolarSystem {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::new())
    }
}

impl MessageClient for SolarSystem {
    fn recv_message(&self, _message: physim_core::messages::Message) {}
}
