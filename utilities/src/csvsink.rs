use physim_attribute::render_element;
use physim_core::{
    Entity,
    messages::MessageClient,
    plugin::{Element, ElementCreator, render::RenderElement},
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    sync::atomic::{AtomicUsize, Ordering},
};
use std::{io::Write, process::exit};

#[render_element(
    name = "csvsink",
    blurb = "Output entity position to comma spaced values"
)]
struct CsvSink {
    iteration: AtomicUsize,
    print_n: usize,
    file: String,
}

impl ElementCreator for CsvSink {
    fn create_element(properties: HashMap<String, Value>) -> Box<Self> {
        let print_n = properties
            .get("print_n")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let file = properties
            .get("file")
            .and_then(|v| v.as_str())
            .unwrap_or("csvsink.csv");
        Box::new(CsvSink {
            iteration: AtomicUsize::new(0),
            print_n,
            file: String::from(file),
        })
    }
}

impl RenderElement for CsvSink {
    fn render(
        &self,
        _config: physim_core::UniverseConfiguration,
        state_recv: std::sync::mpsc::Receiver<Vec<Entity>>,
    ) {
        let res = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file);

        let mut file = match res {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error opening {}: {}", self.file, e);
                std::process::exit(1)
            }
        };

        if let Ok(state) = state_recv.recv() {
            self.iteration.fetch_add(1, Ordering::Relaxed);
            print_state(&mut file, state);
        }

        while let Ok(state) = state_recv.recv() {
            let iteration = self.iteration.fetch_add(1, Ordering::Relaxed);
            if iteration.rem_euclid(self.print_n) == 0 {
                print_state(&mut file, state);
            }
        }
    }
}

#[allow(unused_must_use)]
fn print_state(mut file: &mut File, state: Vec<Entity>) {
    for entity in &state {
        write!(&mut file, "{},{},{},", entity.x, entity.y, entity.z);
    }
    writeln!(&mut file);
}

impl Element for CsvSink {
    fn get_property_descriptions(
        &self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        Ok(HashMap::from([(
            String::from("print_n"),
            String::from("print every n iterations"),
        )]))
    }
}

impl MessageClient for CsvSink {}
