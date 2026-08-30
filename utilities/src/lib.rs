mod bbox;
mod bpm;
mod csvsink;
mod idset;
mod void;
mod wrapper;

use physim_core::register_plugin;

register_plugin!("csvsink", "bbox", "wrapper", "idset", "bpm", "void");
