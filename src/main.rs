use std::{sync::mpsc::sync_channel, thread};

use log::{error, info};
use physim::{octree::simple::Octree, render::renderer, stars::Star, UniverseConfiguration};
fn main() {
    env_logger::init();

    let config = UniverseConfiguration {
        size_x: 1.5,
        size_y: 1.5,
        size_z: 1.5,
    };

    let mut state = Vec::with_capacity(1_000_000);
    let mut ex: Vec<Star> = Vec::with_capacity(1_000_000);

    for _ in 0..1_000_000 {
        state.push(Star::random());
    }

    let (sender, receiver) = sync_channel(100);

    thread::spawn(move || {
        for _ in 0..1000 {
            let new_state: Vec<Star> = state.iter().map(|x| x.update()).collect();

            // 8**5 = 32k,
            let mut tree = Octree::new(5, [0.0, 0.0, 0.4], 1.0);
            for i in new_state.iter() {
                tree.push(*i)
            }

            let centres = tree.get_centres();
            for c in centres {
                ex = tree.get(c).iter().map(|x| **x).collect();
                info!("Sending {:?} stars", ex);
                if let Err(e) = sender.send(ex) {
                    error!("Could not send {}", e);
                    panic!()
                }
                // ex.push(Star::new(c[0], c[1], c[2], 0.03));
            }

            // let new_state = state.clone();

            sender.send(new_state).unwrap();
            //thread::sleep(Duration::from_millis(100));
        }
    });

    renderer(&config, receiver);
    // info!("Finished");
}
