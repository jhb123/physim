use std::{collections::HashMap, sync::Mutex};

use physim_attribute::render_element;
use physim_core::{messages::MessageClient, plugin::{render::RenderElement, Element, ElementCreator}, Entity};
use serde_json::Value;

#[render_element(name = "energysink", blurb = "Do nothing with data")]
struct EnergySink {
    state: Mutex<u64>,
}

impl ElementCreator for EnergySink {
    fn create_element(_properties: HashMap<String, Value>) -> Box<Self> {
        Box::new(EnergySink {
            state: Mutex::new(0),
        })
    }
}

impl RenderElement for EnergySink {
    fn render(
        &self,
        _config: physim_core::UniverseConfiguration,
        state_recv: std::sync::mpsc::Receiver<Vec<Entity>>,
    ) {
        let mut initial_energy = 0.0;
        if let Ok(state) = state_recv.recv() {
            let (potential, kinetic) = calculate_energy(state);
            initial_energy = potential+kinetic;
            let delta = 0.0;
            println!("K = {kinetic:1.6} :: V = {potential:1.6} :: E = {potential:1.6} :: Delta E {delta:1.6}")

        }

        while let Ok(state) =  state_recv.recv() {
            let (potential, kinetic) = calculate_energy(state);
            let energy = kinetic + potential;
            let energy_delta = initial_energy - energy;
            println!("K = {kinetic:1.6} :: V = {potential:1.6} :: E = {energy:1.6} :: Delta E {energy_delta:1.6}")
            // calculate the energy represented by state

        }   
    }
}

fn calculate_energy(state: Vec<Entity>) -> (f32, f32) {
    let mut potential = 0.0;
    let mut kinetic = 0.0;
    for i in 0..state.len() {
        for j in (i+1)..state.len() {
            let r = ((state[i].x - state[j].x).powi(2) + (state[i].y - state[j].y).powi(2) + (state[i].z - state[i].z).powi(2)).sqrt();
            potential = - state[i].mass*state[j].mass / r;
        }
    }

    for entity in state {
        let v =  entity.vx.powi(2) + entity.vy.powi(2) + entity.vz.powi(2);
        kinetic += 0.5 * entity.mass * v;
    }
    (potential, kinetic)
}

impl Element for EnergySink {
    fn set_properties(&self, new_props: HashMap<String, Value>) {
        if let Some(state) = new_props.get("state").and_then(|state| state.as_u64()) {
            *self.state.lock().unwrap() = state
        }
    }

    fn get_property(&self, prop: &str) -> Result<Value, Box<dyn std::error::Error>> {
        match prop {
            "state" => Ok(Value::Number((*self.state.lock().unwrap()).into())),
            _ => Err("No property".into()),
        }
    }

    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::new())
    }
}

impl MessageClient for EnergySink {}