#![feature(iter_intersperse)]
use std::env;

use physim_core::pipeline::Pipeline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mut args = env::args();
    args.next().ok_or("No pipeline description")?;

    let desc: String = args.intersperse(" ".to_string()).collect();
    // "cube n=10000 seed=1 ! star mass=10000.0 x=0.5 y=0.5 radius=0.1 ! star mass=10000.0 x=-0.5 y=-0.5 radius=0.1 ! astro theta=1.5 ! glrender "
    let pipeline = Pipeline::new_from_description(&desc)?;

    pipeline.run();

    Ok(())
}
