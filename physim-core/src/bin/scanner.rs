use std::env;

use physim_core::plugin::{element_db, RegisteredElement};
use yansi::Paint;

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();

    let element_db = element_db();

    let mut elements: Vec<RegisteredElement> = element_db.values().cloned().collect();
    elements.sort_by_key(|k| k.get_lib_path().to_string());

    if args.len() == 1 {
        if elements.is_empty() {
            println!("No elements found")
        }
        for element in elements {
            element.print_element_info_brief();
        }
    } else if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        println!("help")
    } else {
        let element_name = &*args[1];
        if let Some(element) = element_db.get(element_name) {
            element.print_element_info_verbose();
        } else {
            println!("No element called {}", element_name.bold(),)
        }
    }
}
