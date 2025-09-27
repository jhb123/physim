use std::{collections::HashMap, sync::Mutex};

use physim_attribute::transmute_element;
use physim_core::{
    Entity,
    log::info,
    messages::MessageClient,
    plugin::{Element, ElementCreator, transmute::TransmuteElement},
};
use serde_json::Value;

#[derive(PartialEq)]
enum BpmMode {
    Always,
    Exclude,
}

#[transmute_element(name = "bpm", blurb = "Generate Entities")]
struct Bpm {
    inner: Mutex<BpmInner>,
}

struct BpmInner {
    current_frame: u64,
    n: u64,
    m: f64,
    radius: Option<f64>,
    mode: BpmMode,
}

impl TransmuteElement for Bpm {
    fn transmute(&self, data: &mut Vec<Entity>) {
        let mut element = self.inner.lock().unwrap();
        element.current_frame += 1;
        if element.current_frame % element.n != 0 {
            return;
        }

        // find the centre of mass
        // m1*x1 + m2*x2 ... / (m1+m2)
        let mut numerator = [0.0; 3];
        for entity in data.iter() {
            numerator[0] += entity.mass * entity.x;
            numerator[1] += entity.mass * entity.y;
            numerator[2] += entity.mass * entity.z;
        }
        let denominator: f64 = data.iter().map(|e| e.mass).sum();

        let centre_of_mass: [f64; 3] = [
            numerator[0] / denominator,
            numerator[1] / denominator,
            numerator[2] / denominator,
        ];

        if element.mode == BpmMode::Exclude {
            info!("In exclude mode. Checking all elements");
            let closest_r = 0.5;
            if data.iter().any(|e| {
                (e.x - centre_of_mass[0]).abs() < closest_r
                    && (e.y - centre_of_mass[1]).abs() < closest_r
                    && (e.z - centre_of_mass[2]).abs() < closest_r
            }) {
                info!("Skipping this element as there is it is too close to another one");
                return;
            }
        }

        // add the entity at the centre of mass
        let radius = match element.radius {
            Some(r) => r,
            None => data.first().map(|e| e.radius).unwrap_or(0.1),
        };

        let new_entity = Entity::new2(
            centre_of_mass[0],
            centre_of_mass[1],
            centre_of_mass[2],
            element.m,
            radius,
        );

        physim_core::log::info!("adding {:?}", new_entity);

        data.push(new_entity);
    }
}

impl MessageClient for Bpm {}

impl ElementCreator for Bpm {
    fn create_element(props: HashMap<String, Value>) -> Box<Self> {
        let n = props.get("n").map(|x| x.as_u64().unwrap_or(1)).unwrap_or(1);

        let m = props
            .get("m")
            .map(|x| x.as_f64().unwrap_or(1.0))
            .unwrap_or(1.0);

        let radius = props.get("r").map(|x| x.as_f64().unwrap_or(1.0));

        let mode: BpmMode = props
            .get("mode")
            .and_then(|x| x.as_str())
            .map(|mode_str| match mode_str {
                "always" => BpmMode::Always,
                "exclude" => BpmMode::Exclude,
                _ => BpmMode::Always,
            })
            .unwrap_or(BpmMode::Always);

        let inner = BpmInner {
            n,
            m,
            radius,
            current_frame: 0,
            mode,
        };
        Box::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

impl Element for Bpm {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            (
                "n".to_string(),
                "generate an entity on every nth frame".to_string(),
            ),
            ("m".to_string(), "Mass of the entity".to_string()),
            (
                "r".to_string(),
                "Radius of the element. Default to size found in simulation".to_string(),
            ),
            ("mode".to_string(), "valid modes are always, exclude. Always will always make an element, excude will only do it if the entities are spread out enough".to_string())
        ]))
    }
}
