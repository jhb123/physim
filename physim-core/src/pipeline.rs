use std::{
    collections::HashMap,
    error::Error,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use log::{debug, info};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    messages::{Message, MessageBus, MessageClient, MessagePriority},
    plugin::{
        element_db,
        generator::GeneratorElementHandler,
        integrator::{IntegratorElement, IntegratorElementHandler},
        render::RenderElementHandler,
        set_bus,
        transform::TransformElementHandler,
        transmute::{TransmuteElement, TransmuteElementHandler},
        ElementKind, Loadable, RegisteredElement,
    },
    Acceleration, Entity, UniverseConfiguration,
};

use crate::msg;

pub struct Pipeline {
    initialisers: Vec<Arc<GeneratorElementHandler>>,
    synths: Option<Vec<Arc<GeneratorElementHandler>>>,
    transforms: Vec<Arc<TransformElementHandler>>,
    transmutes: Vec<Arc<TransmuteElementHandler>>,
    render: Arc<RenderElementHandler>,
    integrator: Arc<IntegratorElementHandler>,
    timestep: f64,
    iterations: u64,
    bus: Arc<Mutex<MessageBus>>,
}

struct PipelineMessageClient {
    paused: AtomicBool,
    quit: AtomicBool,
}
impl PipelineMessageClient {
    fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            quit: AtomicBool::new(false),
        }
    }
}
impl MessageClient for PipelineMessageClient {
    fn recv_message(&self, message: &Message) {
        if &message.topic == "pipeline" {
            match message.message.as_str() {
                "pause_toggle" => {
                    self.paused.fetch_xor(true, Ordering::SeqCst);
                }
                "quit" => {
                    self.quit.store(true, Ordering::SeqCst);
                }
                _ => {}
            }
        }
    }
}

impl Pipeline {
    pub fn run(self) -> Result<(), String> {
        let config = UniverseConfiguration {
            size_x: 2.0,
            size_y: 1.0,
            size_z: 1.0,
        };

        // cannot be reference since it'd break renderer
        let pipeline_messages = Arc::new(PipelineMessageClient::new());
        match self.bus.lock() {
            Ok(mut bus) => {
                bus.add_client(pipeline_messages.clone());
            }
            Err(_) => {
                eprintln!("Failed to add pipeline to message bus. Message bus poisoned");
                std::process::exit(1)
            }
        }

        self.post_configuration_messages();

        match self.bus.lock() {
            Ok(mut bus) => {
                bus.pop_messages();
            }
            Err(_) => {
                eprintln!("Failed to add pipeline to message bus. Message bus poisoned");
                std::process::exit(1)
            }
        }

        let mut state = Vec::new();
        for el in self.initialisers.iter() {
            state.extend(el.create_entities());
        }
        let mut new_state = Vec::with_capacity(state.capacity());
        for _ in 0..state.len() {
            new_state.push(Entity::default());
        }
        debug!("Set up initial state");

        let msg_flag = Arc::new(AtomicBool::new(true));
        let msg_flag_clone = msg_flag.clone();
        let bus_clone = self.bus.clone();
        let message_thread = thread::spawn(move || {
            debug!("Spawning message thread");
            while msg_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                match bus_clone.lock() {
                    Ok(mut bus) => {
                        bus.pop_messages();
                        thread::sleep(std::time::Duration::from_millis(8));
                    }
                    Err(_) => {
                        eprintln!("Message event loop failed. Message bus poisoned");
                        std::process::exit(1)
                    }
                }
            }
        });

        let (simulation_sender, renderer_receiver) = mpsc::sync_channel(2);
        simulation_sender
            .send(state.clone())
            .expect("The renderer has definitely not been dropped");

        thread::spawn(move || {
            let dt = self.timestep;
            let mut count = 0;
            let transform_fn = |state: &[Entity], accelerations: &mut [Acceleration]| {
                self.transforms
                    .iter()
                    .for_each(|element| element.transform(state, accelerations))
            };

            while count < self.iterations {
                if pipeline_messages.quit.load(Ordering::Relaxed) {
                    break;
                }
                if pipeline_messages.paused.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(1));
                    if let Err(_) = simulation_sender.send(new_state.clone()) {
                        return;
                    };
                    continue;
                } else {
                    count += 1;
                }
                let start = Instant::now();

                self.synths.iter().for_each(|els| {
                    for el in els {
                        let entities = el.create_entities();
                        state.extend(entities.iter());
                        new_state.extend(entities.iter());
                    }
                });

                self.integrator
                    .integrate(&state, &mut new_state, &transform_fn, dt);

                for t in &self.transmutes {
                    t.transmute(&mut new_state);
                }

                state = new_state.clone();
                info!(
                    "Updated state in {} ms. Sending state of len {}",
                    start.elapsed().as_millis(),
                    state.len()
                );
                if let Err(_) = simulation_sender.send(new_state.clone()) {
                    return;
                }
            }
            info!("Finalising pipeline");
            let msg = msg!(1, "pipeline", "finished", MessagePriority::RealTime);
            match self.bus.lock() {
                Ok(mut bus) => bus.post_message(msg),
                Err(_) => {
                    eprintln!("Failed to post exit message");
                    std::process::exit(1)
                }
            }
        });

        self.render.render(config, renderer_receiver);
        msg_flag.store(false, std::sync::atomic::Ordering::Relaxed);
        message_thread
            .join()
            .map_err(|e| format!("Failed join message thread {:?}", e))
    }

    fn post_configuration_messages(&self) {
        debug!("Posting configuration messages");
        self.transforms
            .iter()
            .for_each(|el| el.post_configuration_messages());
        self.initialisers
            .iter()
            .for_each(|el| el.post_configuration_messages());
        if let Some(synths) = &self.synths {
            synths
                .iter()
                .for_each(|el| el.post_configuration_messages());
        }
        self.transmutes
            .iter()
            .for_each(|el| el.post_configuration_messages());
        self.render.post_configuration_messages();
        self.integrator.post_configuration_messages();
        debug!("Finished posting configuration messages");
    }

    pub fn new_from_description(pipeline_description: &str) -> Result<Self, Box<dyn Error>> {
        info!("Parsing: {pipeline_description}");
        let element_descriptions: Vec<&str> = pipeline_description.split_terminator("!").collect();

        let mut builder = PipelineBuilder::new();
        for desc in element_descriptions.into_iter() {
            let (el_name, props) = Self::parse_element_description(desc)?;
            builder = builder.add(&el_name, props)?;
        }
        builder.build()
    }

    pub fn new_from_file(path: &str) -> Result<Pipeline, Box<dyn Error>> {
        let toml_str =
            std::fs::read_to_string(path).map_err(|_| format!("Could not read {path}"))?;
        let config: PipelineConfig = toml::from_str(&toml_str).map_err(|e| {
            // format!("{} {:?}", e.message(), e.span().and_then(|x| Some(toml_str[x]) ))
            match e.span() {
                Some(span) => {
                    let ln_num = toml_str[0..span.start].chars().fold(1, |acc, x| {
                        if x == '\n' {
                            acc + 1
                        } else {
                            acc
                        }
                    });
                    println!("{:?}", span.start);
                    format!("Unexpected character on line {ln_num}: {}", &toml_str[span])
                }
                None => e.message().to_string(),
            }
        })?;
        let mut builder = PipelineBuilder::new();

        // let props = HashMap::from([("dt", serde_json::json!(config.global.dt),])
        let props = HashMap::from([
            ("dt".to_string(), serde_json::json!(config.global.dt)),
            (
                "iterations".to_string(),
                serde_json::json!(config.global.iterations),
            ),
        ]);
        builder = builder.add("global", props)?;

        for (el_name, descriptions) in config.elements {
            for props in descriptions {
                builder = builder.add(&el_name, props)?;
            }
        }
        builder.build()
    }

    fn parse_element_description(
        element_description: &str,
    ) -> Result<(String, HashMap<String, Value>), Box<dyn Error>> {
        let element_description = element_description.trim();
        match element_description.split_once(" ") {
            Some(desc_parts) => {
                info!("Parsing {} {}", desc_parts.0, desc_parts.1);
                let name = desc_parts.0.to_string();

                let mut props = HashMap::new();
                let desc_parts: Vec<&str> = desc_parts.1.split_whitespace().collect();

                // .split_terminator("=").collect::<Vec<&str>>();
                for part in desc_parts {
                    if let Some(x) = part.split_once("=") {
                        if x.0.trim().is_empty() || x.1.trim().is_empty() {
                            return Err(
                                format!("invalid element description: '{name} {part}'").into()
                            );
                        } else if let Ok(val) = serde_json::Value::from_str(x.1.trim()) {
                            props.insert(x.0.trim().to_string(), val);
                        } else {
                            props.insert(x.0.trim().to_string(), Value::String(x.1.to_string()));
                        }
                    } else {
                        return Err(
                            format!("Element description missing: {name} {:?}", part).into()
                        );
                    }
                }
                Ok((name, props))
            }
            None => Ok((element_description.to_string(), HashMap::new())),
        }
    }
}

struct PipelineBuilder {
    initialisers: Vec<Arc<GeneratorElementHandler>>,
    synths: Option<Vec<Arc<GeneratorElementHandler>>>,
    transforms: Vec<Arc<TransformElementHandler>>,
    transmutes: Vec<Arc<TransmuteElementHandler>>,
    render: Option<Arc<RenderElementHandler>>,
    integrator: Option<Arc<IntegratorElementHandler>>,
    element_db: HashMap<String, RegisteredElement>,
    timestep: f64,
    iterations: u64,
    bus: Arc<Mutex<MessageBus>>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        PipelineBuilder {
            initialisers: vec![],
            synths: None,
            transforms: vec![],
            transmutes: vec![],
            render: None,
            integrator: None,
            element_db: element_db(),
            timestep: 0.000001,
            iterations: 10000,
            bus: Arc::new(Mutex::new(MessageBus::new())),
        }
    }

    pub fn add(
        mut self,
        el_name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<Self, Box<dyn Error>> {
        if el_name == "global" {
            if let Some(x) = properties.get("dt").and_then(|x| x.as_f64()) {
                self.timestep = x;
            }
            if let Some(x) = properties.get("iterations").and_then(|x| x.as_u64()) {
                self.iterations = x;
            }
            return Ok(self);
        }

        let element_data = self
            .element_db
            .get(el_name)
            .ok_or(format!("{el_name} is not a registered element"))?;

        unsafe { set_bus(element_data, self.bus.clone())? };

        match element_data.get_element_kind() {
            ElementKind::Initialiser => {
                let element =
                    GeneratorElementHandler::load(element_data.get_lib_path(), el_name, properties)
                        .map_err(|_| "Failed to load initialiser element")?;
                self.add_element_to_bus(element.clone());
                self.initialisers.push(element);
            }
            ElementKind::Transform => {
                let element =
                    TransformElementHandler::load(element_data.get_lib_path(), el_name, properties)
                        .map_err(|_| "Failed to load transform element")?;
                self.add_element_to_bus(element.clone());
                self.transforms.push(element);
            }
            ElementKind::Render => {
                let element =
                    RenderElementHandler::load(element_data.get_lib_path(), el_name, properties)
                        .map_err(|_| "Failed to load transform element")?;
                self.add_element_to_bus(element.clone());
                self.render = Some(element);
            }
            ElementKind::Synth => {
                let element =
                    GeneratorElementHandler::load(element_data.get_lib_path(), el_name, properties)
                        .map_err(|_| "Failed to load synth element")?;
                self.add_element_to_bus(element.clone());
                match self.synths.as_mut() {
                    Some(els) => {
                        els.push(element);
                    }
                    None => {
                        self.synths.replace(vec![element]);
                    }
                }
            }
            ElementKind::Transmute => {
                let element =
                    TransmuteElementHandler::load(element_data.get_lib_path(), el_name, properties)
                        .map_err(|_| "Failed to load transmute element")?;
                self.add_element_to_bus(element.clone());
                self.transmutes.push(element);
            }
            ElementKind::Integrator => {
                let element = IntegratorElementHandler::load(
                    element_data.get_lib_path(),
                    el_name,
                    properties,
                )
                .map_err(|_| "Failed to load transmute element")?;
                self.add_element_to_bus(element.clone());
                self.integrator = Some(element);
            }
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Pipeline, Box<dyn Error>> {
        if self.integrator.is_none() {
            Err("No integrator defined in pipeline".into())
        } else if self.render.is_none() {
            Err("No renderer defined in pipeline".into())
        } else if self.transforms.is_empty() && self.transmutes.is_empty() {
            Err("No transforms defined in pipeline".into())
        } else {
            let transforms = self.transforms;
            let transmutes = self.transmutes;
            Ok(Pipeline {
                initialisers: self.initialisers,
                synths: self.synths,
                transforms,
                transmutes,
                render: self.render.expect("Checked just above"),
                integrator: self.integrator.expect("Checked just above"),
                timestep: self.timestep,
                iterations: self.iterations,
                bus: self.bus,
            })
        }
    }

    fn add_element_to_bus(&self, element: Arc<dyn MessageClient>) {
        match self.bus.lock() {
            Ok(mut b) => b.add_client(element.clone()),
            Err(_) => {
                eprintln!("Failed to add element to message bus. Message bus poisoned");
                std::process::exit(1)
            }
        }
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize, Debug)]
struct PipelineConfig {
    global: GlobalOptions,
    elements: HashMap<String, Vec<HashMap<String, Value>>>,
}

// #[derive(Deserialize, Debug)]
// struct ElementConfig {
//     elements: HashMap<String, Vec<HashMap<String, Value>>>,
//     branches:
// }

#[derive(Deserialize, Debug)]
struct GlobalOptions {
    dt: f64,
    iterations: u64,
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse() {}
}
