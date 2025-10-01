#![feature(str_from_raw_parts)]

#[cfg(feature = "crashers")] // codespell:ignore crashers
mod crashers; // codespell:ignore crashers
mod energysink;

use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use physim_attribute::{
    initialise_state_element, render_element, synth_element, transform_element, transmute_element,
};
use physim_core::{
    Acceleration, Entity,
    log::info,
    messages::{MessageClient, MessagePriority},
    msg,
    plugin::{
        Element, ElementCreator, generator::GeneratorElement, render::RenderElement,
        transform::TransformElement, transmute::TransmuteElement,
    },
    post_bus_msg, register_plugin,
};
use rand::Rng;
use serde_json::Value;

#[cfg(not(feature = "crashers"))] // codespell:ignore crashers
register_plugin!(
    "randsynth",
    "debug",
    "fakesink",
    "msgdebug",
    "void",
    "energysink"
);
#[cfg(feature = "crashers")] // codespell:ignore crashers
register_plugin!(
    "randsynth",
    "debug",
    "fakesink",
    "msgdebug",
    "void",
    "energysink",
    "crashtransform"
);

#[synth_element(name = "randsynth", blurb = "Generate a random entity")]
struct RandSynth {
    active: AtomicBool,
}

impl ElementCreator for RandSynth {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {
            active: AtomicBool::new(false),
        })
    }
}

impl GeneratorElement for RandSynth {
    fn create_entities(&self) -> Vec<Entity> {
        if self.active.load(Ordering::Relaxed) {
            let e = Entity::new2(
                rand::rng().random_range(-1.0..1.0),
                rand::rng().random_range(-1.0..1.0),
                rand::rng().random_range(-1.0..1.0),
                f64::log10(rand::rng().random_range(1.0..6.0)),
                0.03,
            );
            vec![e]
        } else {
            vec![]
        }
    }
}

impl Element for RandSynth {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

impl MessageClient for RandSynth {
    fn recv_message(&self, message: physim_core::messages::Message) {
        if &message.topic == "keyboard.press" {
            match message.message.as_str() {
                "t" => self.active.fetch_xor(true, Ordering::Relaxed),
                _ => false,
            };
        }
    }
}

#[transform_element(name = "debug", blurb = "Pass through data with no effect")]
pub struct DebugTransform {}

impl TransformElement for DebugTransform {
    fn transform(&self, _: &[Entity], acceleration: &mut [Acceleration]) {
        info!("Debug transform");
        for a in acceleration {
            *a += Acceleration::default();
        }
        let msg1 = msg!(self, "debugplugin", "transformed", MessagePriority::Low);
        post_bus_msg!(msg1);
    }

    fn new(_properties: HashMap<String, Value>) -> Self {
        DebugTransform {}
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

// impl MessageClient for DebugTransform{}

impl MessageClient for DebugTransform {}

#[render_element(name = "fakesink", blurb = "Do nothing with data")]
struct FakeSink {}

impl ElementCreator for FakeSink {
    fn create_element(_properties: HashMap<String, Value>) -> Box<Self> {
        info!("Creating FakeSink");
        Box::new(FakeSink {})
    }
}

impl RenderElement for FakeSink {
    fn render(
        &self,
        _config: physim_core::UniverseConfiguration,
        state_recv: std::sync::mpsc::Receiver<Vec<Entity>>,
    ) {
        while state_recv.recv().is_ok() {
            info!("Fake Rendering!");
        }
    }
}

impl Element for FakeSink {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::new())
    }
}

impl MessageClient for FakeSink {}

#[initialise_state_element(name = "msgdebug", blurb = "Print messages")]
struct MessageDebug {}

impl GeneratorElement for MessageDebug {
    fn create_entities(&self) -> Vec<Entity> {
        vec![]
    }
}

impl ElementCreator for MessageDebug {
    fn create_element(_: HashMap<String, Value>) -> Box<Self> {
        Box::new(Self {})
    }
}

impl Element for MessageDebug {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([]))
    }
}

impl MessageClient for MessageDebug {
    fn recv_message(&self, message: physim_core::messages::Message) {
        println!(
            "[MSGDEBUG] Priority: {:?} - topic {} - message: {}",
            message.priority, message.topic, message.message
        )
    }
}

#[transmute_element(name = "void", blurb = "Destroy Entities")]
struct Void {
    inner: Mutex<VoidInner>,
}

struct VoidInner {
    lim: f64,
}

impl TransmuteElement for Void {
    fn transmute(&self, data: &mut Vec<Entity>) {
        let lim = self.inner.lock().unwrap().lim;
        data.retain(|entity| entity.x.abs() < lim && entity.y.abs() < lim);
    }
}

impl MessageClient for Void {}

impl ElementCreator for Void {
    fn create_element(props: HashMap<String, Value>) -> Box<Self> {
        let lim = props
            .get("lim")
            .map(|x| x.as_f64().unwrap_or(1.0))
            .unwrap_or(1.0);
        let inner = VoidInner { lim };
        Box::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

impl Element for Void {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([(
            "lim".to_string(),
            "Maximum distance from origin in x,y, or z an entity can be s".to_string(),
        )]))
    }
}
