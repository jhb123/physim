#![feature(str_from_raw_parts)]

use physim_core::register_plugin;

mod impulse;
mod shm;

register_plugin!("shm", "impulse");
