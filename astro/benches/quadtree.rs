#![feature(test)]

extern crate test;
use astro::quadtree::QuadTree;
use bumpalo::Bump;
use physim_core::Entity;
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use test::Bencher;

fn push_benchmarks(num_entities: usize, b: &mut Bencher) {
    let mut state = Vec::with_capacity(num_entities);
    let mut rng = ChaCha8Rng::seed_from_u64(0);

    for _ in 0..num_entities {
        state.push(Entity::random(&mut rng));
    }

    b.iter(|| {
        let arena = Bump::new();
        {
            let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena);
            for star in state.iter() {
                tree.push(*star);
            }
        }
        drop(arena);
    });
}

#[bench]
fn psuh_100(b: &mut Bencher) {
    push_benchmarks(100, b);
}

#[bench]
fn push_1000(b: &mut Bencher) {
    push_benchmarks(1_000, b);
}

#[bench]
fn push_10_000(b: &mut Bencher) {
    push_benchmarks(10_000, b);
}

#[bench]
fn push_100_000(b: &mut Bencher) {
    push_benchmarks(100_000, b);
}

#[bench]
fn push_1_000_000(b: &mut Bencher) {
    push_benchmarks(1_000_000, b);
}
