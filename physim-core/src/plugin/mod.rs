use std::{collections::HashMap, env, path::Path};

use generator::{ElementConfigurationHandler, GeneratorElementHandler};
use render::RenderElementHandler;
use transform::TransformElementHandler;
use yansi::Paint;

pub mod generator;
pub mod render;
pub mod synth;
pub mod transform;

#[derive(Debug)]
#[repr(C)]
pub enum ElementKind {
    Initialiser,
    Transform,
    Render,
    Synth,
}

// set by library authors, determined at compile time
#[derive(Debug)]
#[repr(C)]
pub struct ElementMeta {
    kind: ElementKind,
    name: String,
    plugin: String,
    version: String,
    license: String,
    author: String,
    blurb: String,
    repo: String,
}

impl ElementMeta {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ElementKind,
        name: &str,
        plugin: &str,
        version: &str,
        license: &str,
        author: &str,
        blurb: &str,
        repo: &str,
    ) -> Self {
        Self {
            kind,
            name: name.to_string(),
            plugin: plugin.to_string(),
            version: version.to_string(),
            license: license.to_string(),
            author: author.to_string(),
            blurb: blurb.to_string(),
            repo: repo.to_string(),
        }
    }
}

#[macro_export]
macro_rules! register_plugin {
    ( $( $x:expr ),* ) => {

        #[unsafe(no_mangle)]
        fn register_plugin() -> std::ffi::CString{
            let mut elements = Vec::new();
            $(
                elements.push($x);
            )*
            std::ffi::CString::new(elements.join(",")).unwrap_or_default()
        }
    };
}

// determined at run time
#[derive(Debug)]
pub struct RegisteredElement {
    element_info: ElementMeta,
    pub lib_path: String,
    properties: HashMap<String, String>,
}

impl RegisteredElement {
    fn new(element_info: ElementMeta, lib_path: &str, properties: HashMap<String, String>) -> Self {
        RegisteredElement {
            element_info,
            lib_path: lib_path.to_string(),
            properties,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.element_info.name
    }

    pub fn get_element_kind(&self) -> &ElementKind {
        &self.element_info.kind
    }

    pub fn print_element_info_brief(&self) {
        match self.element_info.kind {
            ElementKind::Initialiser => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_cyan(),
                "initialiser".cyan().dim()
            ),
            ElementKind::Transform => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_green(),
                "transform".green().dim()
            ),
            ElementKind::Render => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_yellow(),
                "renderer".yellow().dim()
            ),
            ElementKind::Synth => println!(
                "{:>10}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_red(),
                "synth".yellow().dim()
            ),
        }
    }

    pub fn print_element_info_verbose(&self) {
        println!("{}", "Overview".bold().underline().bright_blue());
        println!("{:>10} - {}", "Name".bold(), self.element_info.name.green());
        println!(
            "{:>10} - {}",
            "Blurb".bold(),
            self.element_info.blurb.green()
        );
        println!(
            "{:>10} - {:#?}",
            "Kind".bold(),
            self.element_info.kind.green()
        );
        if !self.properties.is_empty() {
            println!();
            println!("{}", "Properties".underline().bold().bright_blue());
        }
        for (key, desc) in self.properties.iter() {
            println!("{:>10} - {}", key.bold(), desc.green());
        }
        println!();
        println!("{}", "Meta data".underline().bold().bright_blue());
        println!(
            "{:>10} - {}",
            "Authors".bold(),
            self.element_info.author.green()
        );
        println!(
            "{:>10} - {}",
            "License".bold(),
            self.element_info.license.green()
        );
        println!(
            "{:>10} - {}",
            "Version".bold(),
            self.element_info.version.green()
        );
        println!(
            "{:>10} - {}",
            "Repository".bold(),
            self.element_info.repo.green()
        );
        println!();
        println!(
            "Loaded from the {} plugin located in {}",
            self.element_info.plugin.green(),
            self.lib_path.green()
        );
    }
}

pub fn get_plugin_dir() -> String {
    env::var("PHYSIM_PLUGIN_DIR").unwrap_or("./".to_string())
}

pub fn discover() -> Vec<RegisteredElement> {
    let mut elements = Vec::new();
    let plugin_dir = get_plugin_dir();
    let plugin_dir = Path::new(&plugin_dir);
    if !plugin_dir.is_dir() {
        return Vec::new();
    }
    for entry in plugin_dir
        .read_dir()
        .expect("read_dir call failed")
        .flatten()
    {
        if let Some(ex) = entry.path().extension().and_then(|x| x.to_str()) {
            if ["dylib", "so", "dll"].contains(&ex) {
                log::info!("Scanning {:?}", entry);
                unsafe {
                    let lib_path = entry.path().to_str().expect("msg").to_string();
                    if let Ok(lib) = libloading::Library::new(&lib_path) {
                        if let Ok(register_plugin) = lib.get::<libloading::Symbol<
                            unsafe extern "C" fn() -> std::ffi::CString,
                        >>(
                            b"register_plugin"
                        ) {
                            let els = register_plugin().into_string().unwrap();
                            for el in els.split(",") {
                                let register_element =
                                        lib.get::<libloading::Symbol<
                                            unsafe extern "C" fn() -> ElementMeta,
                                        >>(
                                            format!("{el}_register").as_bytes()
                                        )
                                        .unwrap();
                                let element_info = register_element();
                                let properties = match element_info.kind {
                                    ElementKind::Initialiser => {
                                        let el: GeneratorElementHandler =
                                            GeneratorElementHandler::load(
                                                &lib_path,
                                                &element_info.name,
                                                HashMap::new(),
                                            )
                                            .unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                    ElementKind::Transform => {
                                        let el = TransformElementHandler::load(
                                            &lib_path,
                                            &element_info.name,
                                            HashMap::new(),
                                        )
                                        .unwrap();
                                        let el = el.lock().unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                    ElementKind::Render => {
                                        let el = RenderElementHandler::load(
                                            &lib_path,
                                            &element_info.name,
                                            HashMap::new(),
                                        )
                                        .unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                    ElementKind::Synth => {
                                        let el = GeneratorElementHandler::load(
                                            &lib_path,
                                            &element_info.name,
                                            HashMap::new(),
                                        )
                                        .unwrap();
                                        el.get_property_descriptions().unwrap()
                                    }
                                };

                                elements.push(RegisteredElement::new(
                                    element_info,
                                    &lib_path,
                                    properties,
                                ));
                            }
                        }
                    };
                }
            }
        }
    }
    elements
}

pub fn discover_map() -> HashMap<String, RegisteredElement> {
    let elements = discover();
    elements
        .into_iter()
        .map(|m| (m.element_info.name.clone(), m))
        .collect()
}
