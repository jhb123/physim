use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use physim_attribute::render_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, render::RenderElement},
};
use serde_json::Value;

#[render_element(name = "energysink", blurb = "Do nothing with data")]
struct EnergySink {
    iteration: AtomicUsize,
    print_n: usize,
    calc_gpe: AtomicBool,
}

impl ElementCreator for EnergySink {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        let print_n = properties
            .get("print_n")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        Box::new(EnergySink {
            iteration: AtomicUsize::new(0),
            print_n,
            calc_gpe: AtomicBool::new(false),
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
            let iteration = self.iteration.fetch_add(1, Ordering::Relaxed);
            let (potential, kinetic) = self.calculate_energy(&state);
            initial_energy = potential + kinetic;
            let delta = 0.0;
            println!(
                "{iteration:>5} :: K = {kinetic:1.6} :: V = {potential:1.6} :: E = {potential:1.6} :: Delta E {delta:1.6}"
            )
        }

        while let Ok(state) = state_recv.recv() {
            let (potential, kinetic) = self.calculate_energy(&state);
            let energy = kinetic + potential;
            let energy_delta = initial_energy - energy;

            let iteration = self.iteration.fetch_add(1, Ordering::Relaxed);
            if iteration.rem_euclid(self.print_n) == 0 {
                println!(
                    "{iteration:>5} :: K = {kinetic:1.6} :: V = {potential:1.6} :: E = {energy:1.6} :: Delta E {energy_delta:1.6}"
                )
            }

            // calculate the energy represented by state
        }
    }
}

impl EnergySink {
    fn calculate_energy(&self, state: &[Entity]) -> (f64, f64) {
        let potential = match self.calc_gpe.load(Ordering::Relaxed) {
            true => calculate_gravitational_potential(state),
            false => 0.0,
        };

        let kinetic = calculate_kinetic_energy(state);

        (potential, kinetic)
    }
}

fn calculate_gravitational_potential(state: &[Entity]) -> f64 {
    let mut potential = 0.0;
    for i in 0..state.len() {
        for j in (i + 1)..state.len() {
            let r = ((state[i].x - state[j].x).powi(2)
                + (state[i].y - state[j].y).powi(2)
                + (state[i].z - state[j].z).powi(2))
            .sqrt();
            potential = -state[i].mass * state[j].mass / r;
        }
    }
    potential
}

fn calculate_kinetic_energy(state: &[Entity]) -> f64 {
    let mut kinetic = 0.0;
    for entity in state {
        let v = entity.vx.powi(2) + entity.vy.powi(2) + entity.vz.powi(2);
        kinetic += 0.5 * entity.mass * v;
    }
    kinetic
}

impl Element for EnergySink {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([(
            String::from("print_n"),
            String::from("print every n iterations"),
        )]))
    }
}

impl MessageClient for EnergySink {
    fn recv_message(&self, message: physim_core::messages::Message) {
        if message.topic == "energysink" && message.message == "gravity" {
            self.calc_gpe.swap(true, Ordering::Relaxed);
        }
    }
}
