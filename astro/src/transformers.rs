use std::collections::HashMap;

use bumpalo::Bump;
use physim_attribute::transform_element;
use physim_core::{ElementInfo, ElementKind, Entity, TransformElement, TransformElementAPI};
use serde_json::Value;

use crate::{Star, quadtree::QuadTree};

#[transform_element("astro")]
pub struct AstroElement {}

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

            let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), 100.0);
            for star_b in star_bs.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            new_state[i] = star_a.suvat(dt, f);
        }
    }

    fn new(_properties: HashMap<String, Value>) -> Self {
        AstroElement {}
    }
}

#[transform_element("simple_astro")]
pub struct SimpleAstroElement {}

impl TransformElement for SimpleAstroElement {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], dt: f32) {
        for (i, star_a) in state.iter().enumerate() {
            let mut f = [0.0; 3];

            for star_b in state.iter() {
                if star_a.get_centre() == star_b.get_centre() {
                    continue;
                }
                let fij = star_a.newtons_law_of_universal_gravitation(star_b);
                f[0] += fij[0];
                f[1] += fij[1];
                f[2] += fij[2];
            }
            new_state[i] = star_a.suvat(dt, f);
        }
    }

    fn new(_properties: HashMap<String, Value>) -> Self {
        SimpleAstroElement {}
    }
}

#[allow(dead_code)]
#[transform_element("debug")]
pub struct DebugTransform {
    state: u64,
}

impl TransformElement for DebugTransform {
    fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], _dt: f32) {
        for (i, e) in state.iter().enumerate() {
            new_state[i] = *e
        }
    }

    fn new(properties: HashMap<String, Value>) -> Self {
        DebugTransform {
            state: properties
                .get("prop")
                .and_then(|x| x.as_u64())
                .unwrap_or_default(),
        }
    }
}
