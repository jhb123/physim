use std::collections::HashMap;

use bumpalo::Bump;
use physim_attribute::transform_element;
use physim_core::{Entity, messages::MessageClient, plugin::transform::TransformElement};
use serde::Serialize;
use serde_json::Value;

use crate::{Star, octree::Octree, quadtree::QuadTree};

#[transform_element(
    name = "astro",
    blurb = "Compute approximate gravitational forces with the Barnes-Hut algorithm (quadtree)"
)]
#[derive(Serialize)]
pub struct AstroElement {
    theta: f32,
    easing_factor: f32,
}

impl TransformElement for AstroElement {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32) {
        // let mut new_state = Vec::with_capacity(state.len());
        let arena = Bump::new();
        let extent = state
            .iter()
            .flat_map(|x| x.get_centre())
            .map(|x| x.abs())
            .reduce(f32::max)
            .unwrap_or(1.0);
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0 * extent, &arena);
        for star in state.iter() {
            tree.push(*star);
        }

        for (i, star_a) in state.iter().enumerate() {
            let mut f = [0.0; 3];

            let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), self.theta);
            for star_b in star_bs.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b, self.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            new_state[i] = star_a.verlet(dt, f);
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let theta = properties
            .get("theta")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0) as f32;

        AstroElement {
            theta,
            easing_factor,
        }
    }

    fn set_properties(&mut self, properties: HashMap<String, Value>) {
        if let Some(theta) = properties.get("theta").and_then(|theta| theta.as_f64()) {
            self.theta = theta as f32
        }

        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            self.easing_factor = easing_factor as f32
        }
    }

    fn get_property(&mut self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "theta" => Ok(serde_json::json!(self.theta)),
            "e" => Ok(serde_json::json!(self.easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&mut self) -> HashMap<String, String> {
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
    blurb = "Compute approximate gravitational forces with the Barnes-Hut algorithm (octree)"
)]
#[derive(Serialize)]
pub struct AstroOctreeElement {
    theta: f32,
    easing_factor: f32,
}

impl TransformElement for AstroOctreeElement {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32) {
        // let mut new_state = Vec::with_capacity(state.len());
        let arena = Bump::new();
        let extent = state
            .iter()
            .flat_map(|x| x.get_centre())
            .map(|x| x.abs())
            .reduce(f32::max)
            .unwrap_or(1.0);
        let mut tree: Octree<'_, Entity> = Octree::new([0.0; 3], 1.0 * extent, &arena);
        for star in state.iter() {
            tree.push(*star);
        }

        for (i, star_a) in state.iter().enumerate() {
            let mut f = [0.0; 3];
            let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), self.theta);
            for star_b in star_bs.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b, self.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            new_state[i] = star_a.verlet(dt, f);
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let theta = properties
            .get("theta")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0) as f32;

        Self {
            theta,
            easing_factor,
        }
    }

    fn set_properties(&mut self, properties: HashMap<String, Value>) {
        if let Some(theta) = properties.get("theta").and_then(|theta| theta.as_f64()) {
            self.theta = theta as f32
        }

        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            self.easing_factor = easing_factor as f32
        }
    }

    fn get_property(&mut self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "theta" => Ok(serde_json::json!(self.theta)),
            "e" => Ok(serde_json::json!(self.easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&mut self) -> HashMap<String, String> {
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

#[transform_element(name = "simple_astro", blurb = "Compute exact gravitational forces")]
pub struct SimpleAstroElement {
    easing_factor: f32,
}

impl TransformElement for SimpleAstroElement {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32) {
        for (i, star_a) in state.iter().enumerate() {
            let mut f = [0.0; 3];

            for star_b in state.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b, self.easing_factor);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            new_state[i] = star_a.verlet(dt, f);
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        let easing_factor = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
            .unwrap_or(1.0) as f32;

        Self { easing_factor }
    }

    fn set_properties(&mut self, properties: HashMap<String, Value>) {
        if let Some(easing_factor) = properties
            .get("e")
            .and_then(|v| v.as_f64())
            .map(|x| x.abs())
        {
            self.easing_factor = easing_factor as f32
        }
    }

    fn get_property(&mut self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "e" => Ok(serde_json::json!(self.easing_factor)),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(&mut self) -> HashMap<String, String> {
        HashMap::from([(
            String::from("e"),
            String::from("Easing factor. Modify G*Ma*Mb*(r-e)^-2. Default=1.0"),
        )])
    }
}

impl MessageClient for SimpleAstroElement {}
