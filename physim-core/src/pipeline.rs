use std::{
    collections::HashMap,
    error::Error,
    str::FromStr,
    sync::{mpsc, Mutex},
    thread,
    time::Instant,
};

use log::info;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    plugin::{
        discover_map, generator::GeneratorElementHandler, render::RenderElementHandler,
        transform::TransformElementHandler, ElementKind, RegisteredElement,
    },
    Entity, UniverseConfiguration,
};

pub struct Pipeline {
    initialisers: Vec<GeneratorElementHandler>,
    synths: Option<Vec<GeneratorElementHandler>>,
    transforms: Mutex<TransformElementHandler>,
    render: RenderElementHandler,
    timestep: f32,
    iterations: u64,
}

impl Pipeline {
    pub fn run(mut self) {
        // cannot be reference since it'd break renderer
        let config = UniverseConfiguration {
            size_x: 2.0,
            size_y: 1.0,
            size_z: 1.0,
        };

        let mut state = Vec::new();
        for el in self.initialisers.iter() {
            state.extend(el.create_entities());
        }
        let mut new_state = Vec::with_capacity(state.capacity());
        for _ in 0..state.len() {
            new_state.push(Entity::default());
        }

        let (simulation_sender, renderer_receiver) = mpsc::sync_channel(10);
        thread::spawn(move || {
            let dt = self.timestep;
            for _ in 0..self.iterations {
                let start = Instant::now();
                if let Ok(element) = self.transforms.lock() {
                    self.synths.iter().for_each(|els| {
                        for el in els {
                            let entities = el.create_entities();
                            state.extend(entities.iter());
                            new_state.extend(entities.iter());
                        }
                    });

                    element.transform(&state, &mut new_state, dt);
                    state = new_state.clone();
                    info!(
                        "Updated state in {} ms. Sending state of len {}",
                        start.elapsed().as_millis(),
                        state.len()
                    );
                    simulation_sender.send(new_state.clone()).unwrap();
                }
            }
        });

        self.render.render(config, renderer_receiver);
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
    initialisers: Vec<GeneratorElementHandler>,
    synths: Option<Vec<GeneratorElementHandler>>,
    transforms: Option<Mutex<TransformElementHandler>>, // maybe will allow more than one of these one day
    render: Option<RenderElementHandler>,
    element_db: HashMap<String, RegisteredElement>, // this will be expanded later to have more types of elements
    timestep: f32,
    iterations: u64,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        PipelineBuilder {
            initialisers: vec![],
            synths: None,
            transforms: None,
            render: None,
            element_db: discover_map(),
            timestep: 0.000001,
            iterations: 10000,
        }
    }

    pub fn add(
        mut self,
        el_name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<Self, Box<dyn Error>> {
        if el_name == "global" {
            if let Some(x) = properties.get("dt").and_then(|x| x.as_f64()) {
                self.timestep = x as f32;
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

        match element_data.get_element_kind() {
            ElementKind::Initialiser => {
                let element =
                    GeneratorElementHandler::load(&element_data.lib_path, el_name, properties)
                        .map_err(|_| "Failed to load initialiser element")?;
                self.initialisers.push(element);
            }
            ElementKind::Transform => {
                let element =
                    TransformElementHandler::load(&element_data.lib_path, el_name, properties)
                        .map_err(|_| "Failed to load transform element")?;
                self.transforms = Some(element);
            }
            ElementKind::Render => {
                let element =
                    RenderElementHandler::load(&element_data.lib_path, el_name, properties)
                        .map_err(|_| "Failed to load transform element")?;
                self.render = Some(element);
            }
            ElementKind::Synth => {
                let element =
                    GeneratorElementHandler::load(&element_data.lib_path, el_name, properties)
                        .map_err(|_| "Failed to load synth element")?;
                match self.synths.as_mut() {
                    Some(els) => {
                        els.push(element);
                    }
                    None => {
                        self.synths.replace(vec![element]);
                    }
                }
            }
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Pipeline, Box<dyn Error>> {
        if self.render.is_none() {
            Err("No renderer defined in pipeline".into())
        } else if self.transforms.is_none() {
            Err("No transforms defined in pipeline".into())
        } else {
            Ok(Pipeline {
                initialisers: self.initialisers,
                synths: self.synths,
                transforms: self.transforms.expect("Checked just above"),
                render: self.render.expect("Checked just above"),
                timestep: self.timestep,
                iterations: self.iterations,
            })
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

#[derive(Deserialize, Debug)]
struct GlobalOptions {
    dt: f32,
    iterations: u64,
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse() {}
}
