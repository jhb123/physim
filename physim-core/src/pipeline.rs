use std::{
    collections::HashMap,
    str::FromStr,
    sync::{mpsc, Mutex},
    thread,
    time::Instant,
};

use log::{info, warn};
use serde_json::Value;

use crate::{
    plugin::{
        discover_map, initialiser::InitialStateElementHandler, render::RenderElementHandler,
        transform::TransformElementHandler, ElementKind, RegisteredElement,
    },
    Entity, UniverseConfiguration,
};

pub struct Pipeline {
    initialisers: Vec<InitialStateElementHandler>,
    transforms: Mutex<TransformElementHandler>,
    render: RenderElementHandler,
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
            let dt = 0.00001;
            for _ in 0..10000 {
                let start = Instant::now();
                if let Ok(element) = self.transforms.lock() {
                    // new_state.clear();
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

    pub fn new_from_description(pipeline_description: &str) -> Result<Self, &'static str> {
        info!("Parsing: {pipeline_description}");
        let element_descriptions: Vec<&str> = pipeline_description.split_terminator("!").collect();

        let mut builder = PipelineBuilder::new();
        for desc in element_descriptions.into_iter() {
            let (el_name, props) = Self::parse_element_description(desc)?;
            builder = builder.add(&el_name, props)?;
        }
        builder.build()
    }

    fn parse_element_description(
        element_description: &str,
    ) -> Result<(String, HashMap<String, Value>), &'static str> {
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
                            warn!("LHS or RHS is zero length in element description: {:?}", x);
                            return Err("Could not parse element properties");
                        } else if let Ok(val) = serde_json::Value::from_str(x.1.trim()) {
                            props.insert(x.0.trim().to_string(), val);
                        } else {
                            warn!("Cannot parse {:?} to a value", x);
                            return Err("Could not parse element properties");
                        }
                    } else {
                        warn!("Element description missing: {name} {:?}", part);
                        return Err("Could not parse element properties");
                    }
                }
                Ok((name, props))
            }
            None => Ok((element_description.to_string(), HashMap::new())),
        }
    }
}

struct PipelineBuilder {
    initialisers: Vec<InitialStateElementHandler>,
    transforms: Option<Mutex<TransformElementHandler>>, // maybe will allow more than one of these one day
    render: Option<RenderElementHandler>,
    element_db: HashMap<String, RegisteredElement>, // this will be expanded later to have more types of
                                                    // elements
}

impl PipelineBuilder {
    pub fn new() -> Self {
        PipelineBuilder {
            initialisers: vec![],
            transforms: None,
            render: None,
            element_db: discover_map(),
        }
    }

    pub fn add(
        mut self,
        el_name: &str,
        properties: HashMap<String, Value>,
    ) -> Result<Self, &'static str> {
        let element_data = self
            .element_db
            .get(el_name)
            .ok_or("Could not find element name")?;

        match element_data.get_element_kind() {
            ElementKind::Initialiser => {
                let element =
                    InitialStateElementHandler::load(&element_data.lib_path, el_name, properties)
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
        }
        Ok(self)
    }

    pub fn build(self) -> Result<Pipeline, &'static str> {
        if self.render.is_none() {
            Err("No renderer defined in pipeline")
        } else if self.transforms.is_none() {
            Err("No transforms defined in pipeline")
        } else {
            Ok(Pipeline {
                initialisers: self.initialisers,
                transforms: self.transforms.expect("Checked just above"),
                render: self.render.expect("Checked just above"),
            })
        }
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse() {}
}
