use std::collections::HashMap;

use physim_attribute::transmute_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, transmute::TransmuteElement},
};
use serde_json::Value;

#[transmute_element(name = "bbox", blurb = "Reflect entities at bounding box")]
struct BBox {
    xlim: f64,
    ylim: f64,
    zlim: f64,
}

impl TransmuteElement for BBox {
    fn transmute(&self, data: &mut Vec<Entity>) {
        for e in data.iter_mut() {
            if e.x.abs() > self.xlim {
                e.vx *= -1.0
            }
            if e.y.abs() > self.ylim {
                e.vy *= -1.0
            }
            if e.z.abs() > self.zlim {
                e.vz *= -1.0
            }
        }
    }
}

impl MessageClient for BBox {}

impl ElementCreator for BBox {
    fn create_element(props: HashMap<String, Value>) -> Box<Self> {
        let xlim = props
            .get("xlim")
            .map(|x| x.as_f64().unwrap_or(1.0))
            .unwrap_or(1.0);
        let ylim = props
            .get("ylim")
            .map(|x| x.as_f64().unwrap_or(1.0))
            .unwrap_or(1.0);
        let zlim = props
            .get("zlim")
            .map(|x| x.as_f64().unwrap_or(1.0))
            .unwrap_or(1.0);
        Box::new(Self { xlim, ylim, zlim })
    }
}

impl Element for BBox {
    fn set_properties(&self, _new_props: HashMap<String, Value>) {}

    fn get_property(&self, _prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            (
                "xlim".to_string(),
                "Maximum distance from origin in x".to_string(),
            ),
            (
                "ylim".to_string(),
                "Maximum distance from origin in y".to_string(),
            ),
            (
                "zlim".to_string(),
                "Maximum distance from origin in z".to_string(),
            ),
        ]))
    }
}
