use physim_core::register_plugin;

register_plugin!("euler", "verlet", "rk4");

mod euler;
mod rk4;
mod verlet;
