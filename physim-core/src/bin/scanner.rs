use std::env;

use physim_core::plugin::{discover, get_plugin_dir};
use yansi::Paint;

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();

    let elements = discover();
    if args.len() == 1 {
        if elements.is_empty() {
            println!("No elements found in {}", get_plugin_dir())
        }
        for element in elements {
            element.print_element_info_brief();
        }
    } else if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        println!("help")
    } else {
        let element_name = &*args[1];
        if let Some(element) = elements.iter().find(|el| el.get_name() == element_name) {
            element.print_element_info_verbose();
        } else {
            println!(
                "No element called {} found in {}",
                element_name.bold(),
                get_plugin_dir()
            )
        }
    }
}

// fn mutate(v: *mut i32,n: usize,c: usize) {
//     let v = unsafe { std::slice::from_raw_parts_mut(v, n) };
//     v[0] = 2;
//     println!("{:?}",v);

// }
