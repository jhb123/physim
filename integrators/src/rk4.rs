use std::{collections::HashMap, sync::Mutex};

use physim_attribute::integrator_element;
use physim_core::{
    Entity, Force,
    messages::MessageClient,
    plugin::{Element, ElementCreator, integrator::IntegratorElement},
};
use serde_json::Value;

#[integrator_element(
    name = "rk4",
    blurb = "Evaluate evolution with time using Rk4 integration"
)]

struct Rk4 {
    _inner: Mutex<InnerRk4>,
}

struct InnerRk4 {
    _step: u8,
}

impl IntegratorElement for Rk4 {
    // entity can't be dereferenced
    fn integrate(
        &self,
        _entities: &[physim_core::Entity],
        _new_state: &mut [physim_core::Entity],
        _force_fn: &dyn Fn(&[Entity], &mut [Force]),
        _dt: f64,
    ) {
    }
}

impl MessageClient for Rk4 {}

impl ElementCreator for Rk4 {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            _inner: Mutex::new(InnerRk4 { _step: 0 }),
        })
    }
}

impl Element for Rk4 {
    fn set_properties(&self, _: HashMap<String, Value>) {}

    fn get_property(&self, _: &str) -> Result<Value, Box<dyn std::error::Error>> {
        Err("No property".into())
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}
