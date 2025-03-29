#![feature(mpmc_channel)]
use std::{sync::mpsc, thread, time::Instant};

use log::info;
use physim_core::{Entity, UniverseConfiguration};

#[allow(unused_assignments)]
fn main() {
    env_logger::init();

    let path = "/Users/josephbriggs/repos/physim/target/release/libastro.dylib";
    // physim_core::discover()
    let properties = serde_json::json!({ "prop": 0, "b": "2" }).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element = physim_core::TransformElementHandler::load(path, "astro", properties).unwrap();

    let properties = serde_json::json!({ "n": 100000, "seed": 0 }).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let mut cube_maker =
        physim_core::InitialStateElementHandler::load(path, "cube", properties).unwrap();

    let properties =
        serde_json::json!({ "mass": 100000.0, "x": -0.5, "y": 0.5, "radius": 0.25 }).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let mut star_maker1 =
        physim_core::InitialStateElementHandler::load(path, "star", properties).unwrap();

    let properties =
        serde_json::json!({ "mass": 100000.0, "x": 0.5, "y": -0.5, "radius": 0.25 }).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let mut star_maker2 =
        physim_core::InitialStateElementHandler::load(path, "star", properties).unwrap();

    // parse command line arguments to construct the pipeline.

    let config = UniverseConfiguration {
        size_x: 2.0,
        size_y: 1.0,
        size_z: 1.0,
    };

    // prepare the initial condition
    let mut state = cube_maker.create_entities();
    let s1 = star_maker1.create_entities();
    let s2 = star_maker2.create_entities();

    state.extend(s1);
    state.extend(s2);

    let mut new_state: Vec<Entity> = Vec::with_capacity(state.capacity());
    for _ in 0..state.len() {
        new_state.push(Entity::default());
    }

    // let (input_sender, simulation_receiver) = mpmc::channel();
    let (simulation_sender, renderer_receiver) = mpsc::sync_channel(10);

    // let sender_1 = input_sender.clone();
    // thread::spawn(move || {
    //     loop {
    //         thread::sleep(Duration::from_millis(1000));
    //         let new_stars = vec![
    //             Entity::new2(rng.random_range(-1.00..1.00), rng.random_range(-1.00..1.00), rng.random_range(-0.00..1.00), 100000.0, 0.1)];
    //             sender_1.send(new_stars);
    //     }
    // });

    // let sender_2 = input_sender.clone();
    // thread::spawn(move || {
    //     loop {
    //         thread::sleep(Duration::from_millis(1000));
    //         let new_stars = vec![Entity::new2(0.0, -0.5, 0.5, 100000.0, 0.1)];
    //         sender_2.send(new_stars);
    //     }
    // });

    // simulation loop
    thread::spawn(move || {
        let dt = 0.00001;
        for _ in 0..10000 {
            let start = Instant::now();
            // if let Ok(data) = simulation_receiver.try_recv() {
            //     state.extend(data);
            // }
            if let Ok(element) = element.lock() {
                // new_state.clear();
                element.transform(&state, &mut new_state, dt);
                // new_state.resize(state.len(), Entity::default());
                state = new_state.clone();
                info!(
                    "Updated state in {} ms. Sending state of len {}",
                    start.elapsed().as_millis(),
                    state.len()
                );
                simulation_sender.send(new_state.clone()).unwrap();
            }
        }
    });

    let properties = serde_json::json!({ "prop": 10, "b": "2" }).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let path = "/Users/josephbriggs/repos/physim/target/release/libglrender.dylib";

    let mut render_element =
        physim_core::RenderElementHandler::load(path, "glrender", properties).unwrap();
    render_element.render(config, renderer_receiver);

    // renderer(&config, renderer_receiver);
    info!("Finished");
}
