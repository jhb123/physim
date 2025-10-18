use std::{
    collections::HashMap,
    env,
    error::Error,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use libloading::{Library, Symbol};
use terminal_colorsaurus::{theme_mode, QueryOptions, ThemeMode};
use yansi::Paint;

use crate::{
    messages::MessageClient,
    plugin::{
        host_alloc_string, setup_plugin_logger, transform::TransformElementHandler, Element,
        ElementKind, ElementMeta, LibLoader, Loadable, PluginGetMetaFn, RegisterPluginFn,
    },
};

const PHYSIM_PLUGIN_LOADER_RUSTC_VERSION: &str = env!("ABI_INFO");

static THEME_MODE: LazyLock<ThemeMode> =
    LazyLock::new(|| theme_mode(QueryOptions::default()).unwrap());

// determined at run time
#[derive(Debug, Clone)]
pub struct RegisteredElement {
    element_info: ElementMeta,
    lib_path: String,
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

    pub fn get_lib_path(&self) -> &str {
        &self.lib_path
    }

    pub fn get_name(&self) -> &str {
        &self.element_info.name
    }

    pub fn get_element_kind(&self) -> ElementKind {
        self.element_info.kind
    }

    pub fn print_element_info_brief(&self) {
        match *THEME_MODE {
            ThemeMode::Dark => self.print_element_info_brief_dark(),
            ThemeMode::Light => self.print_element_info_brief_light(),
        };
    }

    fn print_element_info_brief_light(&self) {
        match self.element_info.kind {
            ElementKind::Initialiser => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().cyan(),
                "initialiser".cyan()
            ),
            ElementKind::Transform => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().green(),
                "transform".green()
            ),
            ElementKind::Render => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().yellow(),
                "renderer".yellow()
            ),
            ElementKind::Synth => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().red(),
                "synth".red()
            ),
            ElementKind::Transmute => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().black(),
                "transmute".black()
            ),
            ElementKind::Integrator => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.magenta(),
                self.element_info.name.bold().blue(),
                "integrator".blue()
            ),
        }
    }

    fn print_element_info_brief_dark(&self) {
        match self.element_info.kind {
            ElementKind::Initialiser => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_cyan(),
                "initialiser".cyan().dim()
            ),
            ElementKind::Transform => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_green(),
                "transform".green().dim()
            ),
            ElementKind::Render => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_yellow(),
                "renderer".yellow().dim()
            ),
            ElementKind::Synth => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_red(),
                "synth".red().dim()
            ),
            ElementKind::Transmute => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_white(),
                "transmute".white().dim()
            ),
            ElementKind::Integrator => println!(
                "{:>15}: {} {}",
                self.element_info.plugin.bright_magenta(),
                self.element_info.name.bold().bright_blue(),
                "integrator".blue().dim()
            ),
        }
    }

    pub fn print_element_info_verbose(&self) {
        match *THEME_MODE {
            ThemeMode::Dark => self.print_element_info_verbose_dark(),
            ThemeMode::Light => self.print_element_info_verbose_light(),
        };
    }

    fn print_element_info_verbose_dark(&self) {
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

    fn print_element_info_verbose_light(&self) {
        println!("{}", "Overview".bold().underline().bright_blue());
        println!("{:>10} - {}", "Name".bold(), self.element_info.name.red());
        println!("{:>10} - {}", "Blurb".bold(), self.element_info.blurb.red());
        println!(
            "{:>10} - {:#?}",
            "Kind".bold(),
            self.element_info.kind.red()
        );
        if !self.properties.is_empty() {
            println!();
            println!("{}", "Properties".underline().bold().bright_blue());
        }
        for (key, desc) in self.properties.iter() {
            println!("{:>10} - {}", key.bold(), desc.red());
        }
        println!();
        println!("{}", "Meta data".underline().bold().bright_blue());
        println!(
            "{:>10} - {}",
            "Authors".bold(),
            self.element_info.author.red()
        );
        println!(
            "{:>10} - {}",
            "License".bold(),
            self.element_info.license.red()
        );
        println!(
            "{:>10} - {}",
            "Version".bold(),
            self.element_info.version.red()
        );
        println!(
            "{:>10} - {}",
            "Repository".bold(),
            self.element_info.repo.red()
        );
        println!();
        println!(
            "Loaded from the {} plugin located in {}",
            self.element_info.plugin.red(),
            self.lib_path.red()
        );
    }
}

pub fn element_db() -> HashMap<String, RegisteredElement> {
    let elements = discover();
    for element in &elements {
        if setup_plugin_logger(element).is_err() {
            eprintln!("Plugin doesn't implement setup_logger");
        };
    }

    elements
        .into_iter()
        .map(|m| (m.element_info.name.to_string(), m))
        .collect()
}

fn get_plugin_dirs() -> Vec<PathBuf> {
    let binary_path = env::current_exe().expect("Failed to get current executable path");
    let binary_dir = binary_path
        .parent()
        .expect("Executable has no parent directory")
        .to_path_buf();

    let mut dirs = vec![binary_dir];

    if let Ok(extra_plugin_dir) = env::var("PHYSIM_PLUGIN_DIR") {
        for dir in extra_plugin_dir.split(":") {
            let d = PathBuf::from(dir);
            if d.exists() {
                dirs.push(d);
            }
        }
    }

    dirs
}

fn discover() -> Vec<RegisteredElement> {
    let mut elements = Vec::new();
    let plugin_dirs = get_plugin_dirs();
    for plugin_dir in plugin_dirs {
        log::info!("Scanning for plugins {:?}", plugin_dir);
        for entry in plugin_lib_iter(&plugin_dir) {
            log::debug!("Scanning {:?}", entry);
            let lib_path = entry.path().to_str().expect("msg").to_string();
            unsafe {
                if !validate_plugin_abi(&lib_path) {
                    continue;
                }

                let Ok(lib) = LibLoader::get(&lib_path) else {
                    continue;
                };

                for element_info in get_plugin_meta(&lib) {
                    if let Some(properties) =
                        get_registered_element_properties(&element_info, &lib_path)
                    {
                        elements.push(RegisteredElement::new(element_info, &lib_path, properties));
                    }
                }
            };
        }
    }
    elements
}

fn plugin_lib_iter(plugin_dir: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
    plugin_dir
        .read_dir()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|ex| {
            ex.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|ex| matches!(ex, "dylib" | "so" | "dll"))
        })
}

unsafe fn validate_plugin_abi(lib_path: &str) -> bool {
    let Ok(lib) = LibLoader::get(lib_path) else {
        log::debug!("Could not load {lib_path} as plugin");
        return false;
    };
    let Ok(get_plugin_abi_info) = lib
        .get::<Symbol<unsafe extern "C" fn() -> *const std::os::raw::c_char>>(
            b"get_plugin_abi_info",
        )
    else {
        log::debug!("get_plugin_abi_info not found");
        return false;
    };

    let cstr = std::ffi::CStr::from_ptr(get_plugin_abi_info());
    let rust_version = cstr.to_string_lossy().into_owned();
    // This is basically a hack for C plugins. I think the ABI for
    // C is stable. If you'r making a C plugin, you probably know
    // about all the terrible things that can happen in physim.
    if rust_version == "C" {
        return true;
    }
    let ret = rust_version == PHYSIM_PLUGIN_LOADER_RUSTC_VERSION;
    if !ret {
        eprintln!("{} was built with a different version of the rust compiler or for a different platform. The plugin compiled with {} but physim compiled with {} ",&lib_path, rust_version,PHYSIM_PLUGIN_LOADER_RUSTC_VERSION);
    }
    ret
}

unsafe fn get_plugin_meta(lib: &Library) -> Vec<ElementMeta> {
    let mut element_metas = vec![];
    let Ok(register_plugin) = lib.get::<Symbol<RegisterPluginFn>>(b"register_plugin") else {
        return element_metas;
    };

    log::debug!("calling register_plugin");
    let cstr = std::ffi::CStr::from_ptr(register_plugin());
    let els = cstr.to_string_lossy().into_owned();
    for el in els.split(",") {
        let Ok(register_element) =
            lib.get::<Symbol<PluginGetMetaFn>>(format!("{el}_register").as_bytes())
        else {
            log::warn!("Could not load meta data for {el}");
            continue;
        };
        let element_info_ffi = register_element(host_alloc_string);
        let element_info = ElementMeta::from_ffi_owned(element_info_ffi);
        element_metas.push(element_info);
    }
    element_metas
}

fn get_registered_element_properties(
    element_info: &super::ElementMeta,
    lib_path: &str,
) -> Option<HashMap<String, String>> {
    match element_info.kind {
        super::ElementKind::Transform => {
            log::info!("loading transform");
            let el =
                TransformElementHandler::load(lib_path, &element_info.name, HashMap::new()).ok()?;
            log::debug!("Got props");
            el.get_property_descriptions().ok()
        }
        _ => {
            let el: Arc<MetaElement> =
                super::Loadable::load(lib_path, &element_info.name, HashMap::new()).ok()?;
            el.get_property_descriptions().ok()
        }
    }
}

/// Struct for determining metadata
struct MetaElement {
    instance: Box<dyn Element>,
}
impl Loadable for MetaElement {
    type Item = Box<dyn Element>;

    fn new(instance: Self::Item) -> Self {
        Self { instance }
    }
}

impl Element for MetaElement {
    fn get_property_descriptions(&self) -> Result<HashMap<String, String>, Box<dyn Error>> {
        self.instance.get_property_descriptions()
    }
}

impl MessageClient for MetaElement {}
