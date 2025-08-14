use std::{collections::HashMap, sync::Mutex};

use physim_attribute::transform_element;
use physim_core::{
    Acceleration, Entity, messages::MessageClient, plugin::transform::TransformElement,
};
use serde_json::Value;

enum ShmTransformMode {
    GlobalCentre,
    ParticleCentre,
}

#[transform_element(
    name = "shm",
    blurb = "Make all entities into simple harmonic oscillators"
)]
pub struct ShmTransform {
    inner: Mutex<ShmTransformInner>,
}

struct ShmTransformInner {
    origins: Vec<[f64; 3]>,
    k: f64,
    c: f64,
    mode: ShmTransformMode,
}

impl TransformElement for ShmTransform {
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        let mut inner = self.inner.lock().unwrap();
        match inner.mode {
            ShmTransformMode::GlobalCentre => inner.global_centre_transform(state, accelerations),
            ShmTransformMode::ParticleCentre => {
                inner.particle_centre_transform(state, accelerations)
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let k = properties.get("k").and_then(|x| x.as_f64()).unwrap_or(1.0);
        let c = properties.get("c").and_then(|x| x.as_f64()).unwrap_or(0.0);

        let mode: ShmTransformMode = properties
            .get("mode")
            .and_then(|x| x.as_str())
            .map(|mode_str| match mode_str {
                "centre" => ShmTransformMode::GlobalCentre,
                "particle" => ShmTransformMode::ParticleCentre,
                _ => ShmTransformMode::GlobalCentre,
            })
            .unwrap_or(ShmTransformMode::GlobalCentre);

        ShmTransform {
            inner: Mutex::new(ShmTransformInner {
                origins: vec![],
                k,
                c,
                mode,
            }),
        }
    }

    fn set_properties(&self, _properties: HashMap<String, Value>) {}

    fn get_property(&self, _prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                String::from("k"),
                String::from("Spring constant. Default=1.0"),
            ),
            (
                String::from("c"),
                String::from("Damping coefficient. Default=0.0"),
            ),
            (
                String::from("mode"),
                String::from("Either 'centre' or 'particle'. Defaults to centre"),
            ),
        ])
    }
}

impl MessageClient for ShmTransform {}

impl ShmTransformInner {
    fn global_centre_transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        for (f, entity) in accelerations.iter_mut().zip(state) {
            *f += Acceleration {
                x: (-self.k * entity.x - self.c * entity.vx) / entity.mass,
                y: (-self.k * entity.y - self.c * entity.vy) / entity.mass,
                z: (-self.k * entity.z - self.c * entity.vz) / entity.mass,
            };
        }
    }

    fn particle_centre_transform(&mut self, state: &[Entity], accelerations: &mut [Acceleration]) {
        if self.origins.len() != state.len() {
            self.origins = state.iter().map(|e| [e.x, e.y, e.z]).collect();
        }

        let deltas: Vec<[f64; 3]> = self
            .origins
            .iter()
            .zip(state)
            .map(|(a, b)| [b.x - a[0], b.y - a[1], b.z - a[2]])
            .collect();

        for (f, (delta, entity)) in accelerations.iter_mut().zip(deltas.iter().zip(state)) {
            *f += Acceleration {
                x: (-self.k * delta[0] - self.c * entity.vx) / entity.mass,
                y: (-self.k * delta[1] - self.c * entity.vy) / entity.mass,
                z: (-self.k * delta[2] - self.c * entity.vz) / entity.mass,
            };
        }
    }
}
