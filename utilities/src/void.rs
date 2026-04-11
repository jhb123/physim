use std::collections::HashMap;

use physim_attribute::transmute_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, transmute::TransmuteElement},
};
use serde_json::Value;

#[transmute_element(name = "void", blurb = "Delete entities that leave a bounding box")]
struct Void {
    xlim: f64,
    ylim: f64,
    zlim: f64,
}

impl TransmuteElement for Void {
    fn transmute(&self, data: &mut Vec<Entity>) {
        data.retain(|e| e.x.abs() <= self.xlim && e.y.abs() <= self.ylim && e.z.abs() <= self.zlim);
    }
}

impl MessageClient for Void {}

impl ElementCreator for Void {
    fn create_element(props: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            xlim: props.get("xlim").and_then(|v| v.as_f64()).unwrap_or(1.0),
            ylim: props.get("ylim").and_then(|v| v.as_f64()).unwrap_or(1.0),
            zlim: props
                .get("zlim")
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::MAX),
        })
    }
}

impl Element for Void {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([
            (
                "xlim".to_string(),
                "Delete entities where |x| exceeds this limit (default 1.0)".to_string(),
            ),
            (
                "ylim".to_string(),
                "Delete entities where |y| exceeds this limit (default 1.0)".to_string(),
            ),
            (
                "zlim".to_string(),
                "Delete entities where |z| exceeds this limit (default unbounded)".to_string(),
            ),
        ]))
    }
}
