use std::{os::macos::raw::stat, sync::mpsc::{channel, sync_channel}, thread, time::Duration};

use log::info;
use physim::{render::{self, renderer}, stars::Star, UniverseConfiguration};
fn main() {
    env_logger::init();


    let config = UniverseConfiguration {size_x: 2.0, size_y: 1.0, size_z: 0.0};
    let mut state = Vec::with_capacity(1000);

    for _ in 0..1_000_000 {
        state.push(Star::random());
    }

    let (sender, receiver) = sync_channel(10);

    thread::spawn(move || {
        for i in 0..1000 {
            let new_state: Vec<Star> = state.iter().map(|x| { x.update() }).collect();
            // let new_state = state.clone();
            sender.send(new_state).unwrap();
            //thread::sleep(Duration::from_millis(100));
            println!("Generated new state {i}");
        }
    });

    renderer(&config, receiver);
    info!("Finished");
}
