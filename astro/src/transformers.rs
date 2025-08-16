use std::{collections::HashMap, sync::Mutex};

use bumpalo::Bump;
use physim_attribute::transform_element;
use physim_core::{
    Acceleration, Entity, messages::MessageClient, plugin::transform::TransformElement,
};
use serde_json::Value;

use crate::{Star, octree::Octree, quadtree::QuadTree};

#[transform_element(
    name = "astro",
    blurb = "Compute approximate gravitational accelerations with the Barnes-Hut algorithm (quadtree)"
)]
#[repr(C)]
pub struct AstroElement {
    inner: Mutex<InnerBhElement>,
}

#[repr(C)]
struct InnerBhElement {
    theta: f64,
    easing_factor: f64,
}

impl TransformElement for AstroElement {
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        // let mut new_state = Vec::with_capacity(state.len());
        let arena = Bump::new();
        let extent = state
            .iter()
            .flat_map(|x| x.get_centre())
            .map(|x| x.abs())
            .reduce(f64::max)
            .unwrap_or(1.0);
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0 * extent, &arena);
        for star in state.iter() {
            tree.push(*star);
        }
        let element = self.inner.lock().unwrap();
        for (i, star_a) in state.iter().enumerate() {
            if star_a.fixed {
                continue;
            }
            let mut f = [0.0; 3];

            let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), element.theta);
            for star_b in star_bs.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij =
                    star_a.newtons_law_of_universal_gravitation(star_b, element.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            accelerations[i] += Acceleration {
                x: f[0] / star_a.mass,
                y: f[1] / star_a.mass,
                z: f[2] / star_a.mass,
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let theta = properties
            .get("theta")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0);

        AstroElement {
            inner: Mutex::new(InnerBhElement {
                theta,
                easing_factor,
            }),
        }
    }

    fn set_properties(&self, properties: HashMap<String, Value>) {
        let mut element = self.inner.lock().unwrap();
        if let Some(theta) = properties.get("theta").and_then(|theta| theta.as_f64()) {
            element.theta = theta
        }

        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            element.easing_factor = easing_factor
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let element = self.inner.lock().unwrap();
        match prop {
            "theta" => Ok(serde_json::json!(element.theta)),
            "e" => Ok(serde_json::json!(element.easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                String::from("theta"),
                String::from(
                    "Barnes-Hut parameter. Increase for speed, decrease for accuracy. Default=1.0",
                ),
            ),
            (
                String::from("e"),
                String::from("Easing factor. Modify G*Ma*Mb*(r-e)^-2. Default=1.0"),
            ),
        ])
    }
}

impl MessageClient for AstroElement {}

#[transform_element(
    name = "astro2",
    blurb = "Compute approximate gravitational accelerations with the Barnes-Hut algorithm (octree)"
)]
pub struct AstroOctreeElement {
    inner: Mutex<InnerBhElement>,
}

impl TransformElement for AstroOctreeElement {
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        // let mut new_state = Vec::with_capacity(state.len());
        let arena = Bump::new();
        let extent = state
            .iter()
            .flat_map(|x| x.get_centre())
            .map(|x| x.abs())
            .reduce(f64::max)
            .unwrap_or(1.0);
        let mut tree: Octree<'_, Entity> = Octree::new([0.0; 3], 1.0 * extent, &arena);
        for star in state.iter() {
            tree.push(*star);
        }

        let element = self.inner.lock().unwrap();
        for (i, star_a) in state.iter().enumerate() {
            if star_a.fixed {
                continue;
            }
            let mut f = [0.0; 3];
            let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), element.theta);
            for star_b in star_bs.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij =
                    star_a.newtons_law_of_universal_gravitation(star_b, element.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            accelerations[i] += Acceleration {
                x: f[0] / star_a.mass,
                y: f[1] / star_a.mass,
                z: f[2] / star_a.mass,
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let theta = properties
            .get("theta")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0);

        Self {
            inner: Mutex::new(InnerBhElement {
                theta,
                easing_factor,
            }),
        }
    }

    fn set_properties(&self, properties: HashMap<String, Value>) {
        let mut element = self.inner.lock().unwrap();
        if let Some(theta) = properties.get("theta").and_then(|theta| theta.as_f64()) {
            element.theta = theta
        }

        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            element.easing_factor = easing_factor
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let element = self.inner.lock().unwrap();
        match prop {
            "theta" => Ok(serde_json::json!(element.theta)),
            "e" => Ok(serde_json::json!(element.easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                String::from("theta"),
                String::from(
                    "Barnes-Hut parameter. Increase for speed, decrease for accuracy. Default=1.0",
                ),
            ),
            (
                String::from("e"),
                String::from("Easing factor. Modify G*Ma*Mb*(r-e)^-2. Default=1.0"),
            ),
        ])
    }
}

impl MessageClient for AstroOctreeElement {}

// impl Configurable for

#[transform_element(
    name = "simple_astro",
    blurb = "Compute exact gravitational accelerations"
)]
pub struct SimpleAstroElement {
    inner: Mutex<InnerSimpleAstroElement>,
}

struct InnerSimpleAstroElement {
    easing_factor: f64,
}

impl TransformElement for SimpleAstroElement {
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        let inner = self.inner.lock().unwrap();
        for (i, star_a) in state.iter().enumerate() {
            if star_a.fixed {
                continue;
            }
            let mut f = [0.0; 3];

            for star_b in state.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b, inner.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            accelerations[i] += Acceleration {
                x: f[0] / star_a.mass,
                y: f[1] / star_a.mass,
                z: f[2] / star_a.mass,
            }
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0);

        Self {
            inner: Mutex::new(InnerSimpleAstroElement { easing_factor }),
        }
    }

    fn set_properties(&self, properties: HashMap<String, Value>) {
        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            self.inner.lock().unwrap().easing_factor = easing_factor
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "e" => Ok(serde_json::json!(self.inner.lock().unwrap().easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([(
            String::from("e"),
            String::from("Easing factor. Modify G*Ma*Mb*(r-e)^-2. Default=1.0"),
        )])
    }
}

impl MessageClient for SimpleAstroElement {}
