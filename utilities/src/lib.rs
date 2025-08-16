mod bbox;
mod csvsink;
mod idset;
mod wrapper;

use physim_core::register_plugin;

register_plugin!("csvsink", "bbox", "wrapper", "idset");
