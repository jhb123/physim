use std::{sync::mpsc::sync_channel, thread};

use log::info;
use physim::{render::{renderer}, stars::Star, UniverseConfiguration};
fn main() {
    env_logger::init();


    let config = UniverseConfiguration {size_x: 2.0, size_y: 1.0, size_z: 0.0};
    let mut state = Vec::with_capacity(1000);

    for _ in 0..1_000_000 {
        state.push(Star::random());
    }

    let (sender, receiver) = sync_channel(100);

    thread::spawn(move || {
        for _ in 0..1000 {
            let new_state: Vec<Star> = state.iter().map(|x| { x.update() }).collect();
            // let new_state = state.clone();
            sender.send(new_state).unwrap();
            //thread::sleep(Duration::from_millis(100));
        }
    });

    renderer(&config, receiver);
    info!("Finished");
}
