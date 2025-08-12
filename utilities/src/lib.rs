mod bbox;
mod csvsink;
mod wrapper;

use physim_core::register_plugin;

register_plugin!("csvsink", "bbox", "wrapper");
