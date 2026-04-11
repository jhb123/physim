use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use physim_attribute::synth_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, generator::GeneratorElement},
};
use serde_json::Value;

// ── shared helpers ────────────────────────────────────────────────────────────

/// Stable id derived from a spawn coordinate.
///
/// Photons emitted from the same world-space point always receive the same id,
/// which maps to the same colour slot in the id shader (`id % 12`).  Two
/// source lines at different positions naturally land on different slots
/// without any configuration.
fn spawn_id(x: f64, y: f64) -> usize {
    // Quantise to 1/1000 world-unit steps so floating-point jitter doesn't
    // break equality, then mix the two integers with a multiplicative hash.
    let xi = (x * 1000.0).round() as i64;
    let yi = (y * 1000.0).round() as i64;
    let h = (xi as u64)
        .wrapping_mul(2654435761)
        .wrapping_add(yi as u64)
        .wrapping_mul(2246822519);
    // Mask to the positive i32 range: the GLSL shader receives id as a signed
    // int, so values with the high bit set become negative, making `id % 12`
    // negative and indexing off the end of the colour palette (black).
    (h & 0x7FFF_FFFF) as usize
}

/// Builds photons as parallel rays distributed along the line from `from` to `to`.
///
/// The travel direction is the unit normal to that line segment that points
/// toward the origin (the black hole location). This is computed purely from
/// the geometry of the source line — no knowledge of rs is needed.
///
/// Photons are evenly spaced between `from` and `to`. Each photon's id is
/// derived from its spawn coordinate so that photons from the same position
/// always share a colour in the id shader, regardless of which wavefront they
/// belong to.
fn make_photons(n: usize, from: [f64; 2], to: [f64; 2], z: f64) -> Vec<Entity> {
    // Vector along the source line
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];

    // Two candidate unit normals (90° rotations of the line vector)
    let len = (dx * dx + dy * dy).sqrt();
    let (nx1, ny1) = (-dy / len, dx / len);
    let (nx2, ny2) = (dy / len, -dx / len);

    // Midpoint of the source line
    let mx = (from[0] + to[0]) / 2.0;
    let my = (from[1] + to[1]) / 2.0;

    // Pick the normal whose dot product with (origin − midpoint) is positive,
    // i.e. the one that points toward the origin.
    let (vx, vy) = if nx1 * (-mx) + ny1 * (-my) >= 0.0 {
        (nx1, ny1)
    } else {
        (nx2, ny2)
    };

    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.5
            } else {
                i as f64 / (n - 1) as f64
            };
            let x = from[0] + t * dx;
            let y = from[1] + t * dy;
            Entity {
                x,
                y,
                z,
                vx,
                vy,
                vz: 0.0,
                mass: 0.0,
                radius: 0.005,
                id: spawn_id(x, y),
                fixed: false,
            }
        })
        .collect()
}

fn parse_point(properties: &HashMap<String, Value>, key: &str, default: [f64; 2]) -> [f64; 2] {
    properties
        .get(key)
        .and_then(|v| {
            let arr = v.as_array()?;
            if arr.len() != 2 {
                return None;
            }
            Some([arr[0].as_f64()?, arr[1].as_f64()?])
        })
        .unwrap_or(default)
}

fn wavefront_property_descriptions() -> HashMap<String, String> {
    HashMap::from([
        ("n".to_string(), "Total number of photons".to_string()),
        (
            "from".to_string(),
            "Start point of the source line in world coordinates, e.g. [-0.5, -2.0]".to_string(),
        ),
        (
            "to".to_string(),
            "End point of the source line in world coordinates, e.g. [0.5, -2.0]".to_string(),
        ),
        (
            "z".to_string(),
            "z-coordinate for rendering (equatorial plane depth)".to_string(),
        ),
    ])
}

// ── photon_ring synth ─────────────────────────────────────────────────────────

/// Emits a complete wavefront of massless photons every `period` ticks,
/// creating a repeating train of parallel rays that are deflected by the
/// Schwarzschild geometry.
///
/// Unlike photon_wavefront (which distributes photons one-per-tick across
/// the source line), photon_ring emits all `n` photons simultaneously every
/// `period` ticks.  Multiple wavefronts are therefore in transit at once,
/// making the lensing pattern continuously visible.
#[synth_element(
    name = "photon_ring",
    blurb = "Periodic wavefront train: emits all N rays at once every `period` ticks"
)]
pub struct PhotonRing {
    params: PhotonRingParams,
    tick: AtomicUsize,
}

struct PhotonRingParams {
    n: usize,
    from: [f64; 2],
    to: [f64; 2],
    z: f64,
    period: usize,
}

impl ElementCreator for PhotonRing {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            params: PhotonRingParams {
                n: properties.get("n").and_then(|v| v.as_u64()).unwrap_or(80) as usize,
                from: parse_point(&properties, "from", [-0.5, -0.85]),
                to: parse_point(&properties, "to", [0.5, -0.85]),
                z: properties.get("z").and_then(|v| v.as_f64()).unwrap_or(0.5),
                period: properties
                    .get("period")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(40) as usize,
            },
            tick: AtomicUsize::new(0),
        })
    }
}

impl GeneratorElement for PhotonRing {
    fn create_entities(&self) -> Vec<Entity> {
        let p = &self.params;
        let t = self.tick.fetch_add(1, Ordering::Relaxed);
        if t % p.period == 0 {
            make_photons(p.n, p.from, p.to, p.z)
        } else {
            vec![]
        }
    }
}

impl Element for PhotonRing {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut desc = wavefront_property_descriptions();
        desc.insert(
            "period".to_string(),
            "Ticks between each wavefront emission (default 40)".to_string(),
        );
        Ok(desc)
    }
}

impl MessageClient for PhotonRing {}

// ── photon_wavefront synth ────────────────────────────────────────────────────

/// Trickles massless photons each tick along a source line specified in world
/// coordinates, building up a continuously evolving lensed wavefront.
///
/// Photons are distributed from `from` to `to` and travel in the inward-facing
/// normal direction. `rate` photons are emitted per tick until `n` total have
/// been released. Because successive photons start at different ticks they are
/// at different stages of their geodesic trajectory simultaneously, producing
/// the characteristic bent-wavefront pattern of gravitational lensing.
///
/// No black-hole parameters are needed here.
#[synth_element(
    name = "photon_wavefront",
    blurb = "Wavefront synth: trickles photons each tick along a world-space source line"
)]
pub struct PhotonWavefront {
    params: PhotonWavefrontParams,
    emitted: AtomicUsize,
}

struct PhotonWavefrontParams {
    n: usize,
    from: [f64; 2],
    to: [f64; 2],
    z: f64,
    rate: usize,
}

impl ElementCreator for PhotonWavefront {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            params: PhotonWavefrontParams {
                n: properties.get("n").and_then(|v| v.as_u64()).unwrap_or(200) as usize,
                from: parse_point(&properties, "from", [-0.5, -2.0]),
                to: parse_point(&properties, "to", [0.5, -2.0]),
                z: properties.get("z").and_then(|v| v.as_f64()).unwrap_or(0.5),
                rate: properties.get("rate").and_then(|v| v.as_u64()).unwrap_or(1) as usize,
            },
            emitted: AtomicUsize::new(0),
        })
    }
}

impl GeneratorElement for PhotonWavefront {
    fn create_entities(&self) -> Vec<Entity> {
        let p = &self.params;
        let already = self.emitted.load(Ordering::Relaxed);
        if already >= p.n {
            return vec![];
        }

        let batch = p.rate.min(p.n - already);
        self.emitted.fetch_add(batch, Ordering::Relaxed);

        make_photons(p.n, p.from, p.to, p.z)
            .into_iter()
            .take(batch)
            .collect()
    }
}

impl Element for PhotonWavefront {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut desc = wavefront_property_descriptions();
        desc.insert(
            "rate".to_string(),
            "Photons emitted per simulation tick (default 1)".to_string(),
        );
        Ok(desc)
    }
}

impl MessageClient for PhotonWavefront {}
