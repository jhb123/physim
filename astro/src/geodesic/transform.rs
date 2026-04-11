use std::collections::HashMap;

use physim_attribute::transform_element;
use physim_core::{
    Acceleration, Entity, messages::MessageClient, plugin::transform::TransformElement,
};
use serde_json::Value;

/// Solves the tensor geodesic equation for a massless particle in Schwarzschild spacetime:
///
///   d²xᵘ/dλ² + Γᵘₐᵦ (dxᵃ/dλ)(dxᵝ/dλ) = 0
///
/// Working in the equatorial plane (θ = π/2) with natural units (G = c = 1).
///
/// Non-zero Christoffel symbols of the Schwarzschild metric used:
///   Γᵗₜᵣ =  rs / (2r(r−rs))
///   Γʳₜₜ =  rs(r−rs) / (2r³)
///   Γʳᵣᵣ = −rs / (2r(r−rs))
///   Γʳᶠᶠ = −(r−rs)
///   Γᶠᵣᶠ =  1/r
///
/// The time component ṫ is eliminated using the conserved energy E = (1−rs/r)·ṫ = 1.
/// The null condition ṙ² = 1 − (1−rs/r)·L²/r² is then substituted into the Christoffel
/// sum to cancel the (1−rs/r) denominators analytically. The result is:
///
///   A ≡ r̈ − r·φ̇²  =  −3·rs·L² / (2r⁴)    where L = x·vy − y·vx
///
/// This form contains no metric singularity and is numerically stable at r = rs.
/// Converted to Cartesian:  ẍ = A·(x/r),  ÿ = A·(y/r)
///
/// Derivation sketch:
///   r̈ (from Christoffels, after substituting null+energy conditions)
///     = −rs·L²/(2r⁴) + (1−rs/r)·L²/r³
///   r̈ − r·φ̇² = −rs·L²/(2r⁴) + (1−rs/r)·L²/r³ − L²/r³
///              = −rs·L²/(2r⁴) − rs·L²/r⁴
///              = −3·rs·L²/(2r⁴)
///
/// Only entities with mass = 0 (photons/massless particles) are affected.
/// Fixed entities (e.g. a visual black hole marker) are skipped.
#[transform_element(
    name = "schwarzschild_geodesic",
    blurb = "Geodesic equation for massless particles in Schwarzschild spacetime (gravitational lensing)"
)]
pub struct SchwarzschildGeodesic {
    /// Schwarzschild radius rs = 2GM/c²
    rs: f64,
}

impl TransformElement for SchwarzschildGeodesic {
    fn new(properties: HashMap<String, Value>) -> Self {
        Self {
            rs: properties.get("rs").and_then(|v| v.as_f64()).unwrap_or(0.1),
        }
    }

    fn transform(&self, state: &[Entity], acceleration: &mut [Acceleration]) {
        let rs = self.rs;

        for (entity, acc) in state.iter().zip(acceleration.iter_mut()) {
            // Only massless, non-fixed particles follow null geodesics here
            if entity.mass != 0.0 || entity.fixed {
                continue;
            }

            let r2 = entity.x * entity.x + entity.y * entity.y;
            let r = r2.sqrt();

            // Skip particles at or inside the event horizon to prevent divergence
            if r <= rs || r < 1e-10 {
                continue;
            }

            // Angular momentum L = r²·φ̇ = x·vy − y·vx (conserved along geodesic)
            let ang_mom = entity.x * entity.vy - entity.y * entity.vx;

            // Stable geodesic acceleration (no 1/f singularity):
            //   A ≡ r̈ − r·φ̇² = −3·rs·L² / (2r⁴)
            let big_a = -3.0 * rs * ang_mom * ang_mom / (2.0 * r2 * r2);

            // Project back to Cartesian (φ̈ cross-terms cancel identically)
            acc.x += big_a * entity.x / r;
            acc.y += big_a * entity.y / r;
            // acc.z left unchanged: photons remain in the equatorial plane
        }
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::from([(
            "rs".to_string(),
            "Schwarzschild radius rs = 2GM/c² of the central body".to_string(),
        )])
    }
}

impl MessageClient for SchwarzschildGeodesic {}
