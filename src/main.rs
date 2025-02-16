use std::{sync::mpsc::sync_channel, thread, time::Instant, vec};

use log::{error, info};
use physim::{octree::simple::Octree, quadtree::simple::Quadree, render::renderer, stars::Star, UniverseConfiguration};

#[allow(unused_assignments)]
fn main() {
    env_logger::init();

    let config = UniverseConfiguration {
        size_x: 2.0,
        size_y: 1.0,
        size_z: 1.0,
    };

    let mut state = Vec::with_capacity(1_000_0);
    let mut new_state: Vec<Star> = Vec::with_capacity(1_000_0);

    for _ in 0..1000 {
        state.push(Star::random());
    }

    // Add a super heavy star
    state.push(Star::new2(0.0,0.0, 0.4, 0.0, 0.5));
    // state.push(Star::new(0.0,0.5, 0.4, 0.05));


    let (sender, receiver) = sync_channel(10);

    thread::spawn(move || {
        let dt = 0.01;
        for _ in 0..10000 {
            let start = Instant::now();

            new_state.clear();
            for (i, star_a) in state.iter().enumerate() {
                let mut f = [0.0;3];
                for (j, star_b) in state.iter().enumerate() {
                    if i==j {
                        continue;
                    }
                    let fij = star_a.newtons_law_of_universal_gravitation(star_b);
                    f[0] += fij[0];
                    f[1] += fij[1];
                    f[2] += fij[2];
                }
                new_state.push(star_a.suvat(dt, f));
            }
            

            // let new_state: Vec<Star> = state.iter().map(|x| x.jitter_position()).collect();
            // sender.send(new_state).unwrap();

            // state.iter_mut().for_each(|x| x.jitter_position_inplace());
            // sender.send(state.clone()).unwrap();

            // state.iter_mut().for_each(|x| x.jitter_position());

            // 8**5 = 32k,
            // let mut tree = Quadree::new(3, [0.0, 0.0, 0.5], 0.5);
            // for i in state.iter() {
            //     tree.push(*i)
            // }
            // ex = tree.get([0.0,0.0,0.5]).iter().map(|x| **x).collect();
            // new_state.clear();
            //// TEST CENTRES appear in the right places
            // let centres = tree.get_leaf_centres();
            // for c in centres {
            //     new_state.push(Star::new(c[0],c[1],c[2], 0.05));
            // }
            // state = new_state.clone();
            // sender.send(new_state.clone()).unwrap();

            //// TEST fakes appear in the right places for calculations
            // let fakes = tree.get_fakes([0.0,0.0,0.4]);
            // let mut v = vec![];
            // for f in fakes {
            //     v.push(*f);
            // }
            // if v.len() > 0 {
            //     sender.send(v).unwrap();
            // }

            // let centres = tree.get_leaf_centres();
            // for c in centres {
            //     let stars: Vec<Star> = tree.get_real(c).iter().map(|x| **x).collect();
            //     let fake_stars: Vec<Star> = tree.get_fakes(c).iter().map(|x| **x).collect();
            //     for (i, star_a) in stars.iter().enumerate() {
            //         let mut f = [0.0;3];
            //         for (j, star_b) in stars.iter().enumerate() {
            //             if i==j {
            //                 continue;
            //             }
            //             let fij = physim::newtons_law_of_universal_gravitation(star_a,star_b);
            //             f[0] += fij[0];
            //             f[1] += fij[1];
            //             f[2] += fij[2];
            //         }

            //         for star_b in fake_stars.iter() {
            //             let fij = physim::newtons_law_of_universal_gravitation(star_a,star_b);
            //             f[0] += fij[0];
            //             f[1] += fij[1];
            //             f[2] += fij[2];
            //         }

            //         new_state.push(star_a.suvat(dt, f));
            //     }
            // }
            
            state = new_state.clone();
            println!("Updated state in {} ms. Sending state of len {}", start.elapsed().as_millis(), state.len());
            sender.send(new_state.clone()).unwrap();

        }
    });

    renderer(&config, receiver);
    // info!("Finished");
}
