use std::{sync::mpsc::sync_channel, thread, time::Instant};

use bumpalo::Bump;
use log::info;
use physim::{
    quadtree::second::QuadTree, render::renderer, stars::Star, Entity, UniverseConfiguration,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[allow(unused_assignments)]
fn main() {
    env_logger::init();

    let config = UniverseConfiguration {
        size_x: 2.0,
        size_y: 1.0,
        size_z: 1.0,
    };

    let mut state = Vec::with_capacity(100_000);
    let mut new_state: Vec<Star> = Vec::with_capacity(100_000);
    let mut rng = ChaCha8Rng::seed_from_u64(0);

    for _ in 0..10_000 {
        state.push(Star::random(&mut rng));
    }

    // Add a super heavy star
    state.push(Star::new2(0.0, 0.0, 0.4, 1000.0, 0.1));
    // state.push(Star::new(0.0,0.5, 0.4, 0.05));

    let (sender, receiver) = sync_channel(10);

    thread::spawn(move || {
        let dt = 0.001;
        for _ in 0..10000 {
            let start = Instant::now();

            new_state.clear();
            advanced_simulation(&state, &mut new_state, dt);
            //simple_simulation(&state, &mut new_state, dt);
            state = new_state.clone();
            println!(
                "Updated state in {} ms. Sending state of len {}",
                start.elapsed().as_millis(),
                state.len()
            );
            sender.send(new_state.clone()).unwrap();
        }
    });

    renderer(&config, receiver);
    info!("Finished");
}

#[allow(dead_code)]
fn simple_simulation(state: &[Star], new_state: &mut Vec<Star>, dt: f32) {
    for (i, star_a) in state.iter().enumerate() {
        let mut f = [0.0; 3];
        for (j, star_b) in state.iter().enumerate() {
            if i == j {
                continue;
            }
            let fij = star_a.newtons_law_of_universal_gravitation(star_b);
            f[0] += fij[0];
            f[1] += fij[1];
            f[2] += fij[2];
        }
        new_state.push(star_a.suvat(dt, f));
    }
}

fn advanced_simulation(state: &[Star], new_state: &mut Vec<Star>, dt: f32) {
    let arena = Bump::new();

    let mut tree: QuadTree<'_, Star> = QuadTree::new([0.0; 3], 10.0, &arena);
    for star in state.iter() {
        tree.push(*star);
    }

    for star_a in state.iter() {
        let mut f = [0.0; 3];

        let star_bs = tree.get_leaves_with_resolution(star_a.get_centre(), 0.5);
        for star_b in star_bs.iter() {
            if star_a.get_centre() == star_b.get_centre() {
                continue;
            }
            let fij = star_a.newtons_law_of_universal_gravitation(star_b);
            f[0] += fij[0];
            f[1] += fij[1];
            f[2] += fij[2];
        }
        new_state.push(star_a.suvat(dt, f));
    }
}
