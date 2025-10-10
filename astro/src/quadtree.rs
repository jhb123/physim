use bumpalo::{Bump, boxed};

use crate::Star;

type Link<'a, T> = boxed::Box<'a, QuadTreeNode<'a, T>>;

#[derive(Debug)]
pub struct QuadTree<'a, T>
where
    T: Star,
{
    root: QuadTreeNode<'a, T>,
    arena: &'a Bump,
}

#[derive(Default, Debug)]
struct QuadTreeNode<'a, T>
where
    T: Star,
{
    centre: [f64; 3],
    extent: f64,
    entity: Option<T>,
    children: [Option<Link<'a, T>>; 4],
}

impl<'a, T> QuadTree<'a, T>
where
    T: Star + Default + Copy,
{
    pub fn new(centre: [f64; 3], extent: f64, arena: &'a Bump) -> Self {
        let root = QuadTreeNode::<T>::new(centre, extent);
        Self { root, arena }
    }

    pub fn push(&mut self, item: T) {
        self.root.push(item, 0, self.arena);
    }

    pub fn get_leaves_with_resolution(&self, location: [f64; 3], bh_factor: f64) -> Vec<T> {
        self.root.get_leaves_with_resolution(location, bh_factor)
    }
}

impl<'a, T> QuadTreeNode<'a, T>
where
    T: Star + Default + Copy,
{
    fn new(centre: [f64; 3], extent: f64) -> Self {
        // todo! put an actual implementation here
        Self {
            centre,
            extent,
            entity: None,
            ..Default::default()
        }
    }

    fn push(&mut self, item: T, count: usize, arena: &'a Bump) {
        if count > 64 {
            panic!("Recursion too deep {:?}", item.get_centre())
        };
        match self.entity.as_ref() {
            None => {
                // if there is nothing in the node, put an entity in it.
                self.entity.replace(item);
            }
            Some(current_elem) => {
                if self.children.iter().all(|x| x.is_none())
                    && current_elem
                        .get_centre()
                        .iter()
                        .zip(item.get_centre().iter())
                        .all(|(a, b)| f64::abs(a - b) < 1e-9)
                {
                    let fake_elem =
                        T::fake(item.get_centre(), current_elem.get_mass() + item.get_mass());
                    self.entity.replace(fake_elem);
                    return;
                }

                // replace the current entity with a new one. take the current one and put it into a child
                let centre_of_mass = current_elem.centre_of_mass(&item);
                let fake_elem = T::fake(centre_of_mass, current_elem.get_mass() + item.get_mass());
                let current_elem = self
                    .entity
                    .replace(fake_elem)
                    .expect("We just checked this is true in the match statement");

                // if this node has no children, the current element is a "real" on and should be added
                // to the corresponding child node
                if self.children.iter().all(|x| x.is_none()) {
                    let item_pos = current_elem.get_centre();
                    let idx = self.get_octant_id(item_pos);
                    // let mut new_node = Box::new_in(QuadTreeNode::<T>, arena);
                    let mut new_node = boxed::Box::new_in(
                        QuadTreeNode::<T>::new(
                            self.get_octant_id_centre(current_elem.get_centre()),
                            self.extent / 2.0,
                        ),
                        arena,
                    );
                    new_node.entity.replace(current_elem);
                    self.children[idx] = Some(new_node);
                }

                // insert the new element
                let item_pos = item.get_centre();
                let idx = self.get_octant_id(item_pos);
                match self.children[idx].as_mut() {
                    Some(node) => {
                        node.as_mut().push(item, count + 1, arena);
                    }
                    None => {
                        let mut new_node = boxed::Box::new_in(
                            QuadTreeNode::<T>::new(
                                self.get_octant_id_centre(item.get_centre()),
                                self.extent / 2.0,
                            ),
                            arena,
                        );
                        new_node.entity.replace(item);
                        self.children[idx] = Some(new_node);
                    }
                }
            }
        }
    }

    fn get_leaves_with_resolution(&self, location: [f64; 3], bh_factor: f64) -> Vec<T> {
        let mut result = Vec::with_capacity(100);
        let mut stack = Vec::with_capacity(100);
        stack.push(self);

        while let Some(node) = stack.pop() {
            if let Some(e) = node.entity {
                let r = ((location[0] - node.centre[0]).powi(2)
                    + (location[1] - node.centre[1]).powi(2)
                    + (location[2] - node.centre[2]).powi(2))
                .sqrt();
                if node.extent / r < bh_factor {
                    result.push(e);
                    continue;
                }
            }
            if node.entity.is_none() {
                // this only happens with empty trees
                continue;
            }
            if node.children.iter().all(|x| x.is_none()) {
                result.push(node.entity.expect("This can is already handled above"));
            } else {
                for child in node.children.iter().flatten() {
                    stack.push(child)
                }
            }
        }
        result
    }

    fn get_octant_id(&self, item_pos: [f64; 3]) -> usize {
        let x_bit = (item_pos[0] > self.centre[0]) as usize;
        let y_bit = (item_pos[1] > self.centre[1]) as usize;
        x_bit | (y_bit << 1)
    }

    fn get_octant_id_centre(&self, item_pos: [f64; 3]) -> [f64; 3] {
        let id = self.get_octant_id(item_pos);
        match id {
            0 => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2],
            ],
            1 => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2],
            ],
            2 => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2],
            ],
            3 => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2],
            ],
            _ => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use physim_core::Entity;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_empty_tree() {
        let arena = Bump::new();
        let tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena);

        let leaves = tree.get_leaves_with_resolution([0.0; 3], 0.5);
        assert_eq!(leaves.len(), 0, "Empty tree should return no leaves");
    }

    #[test]
    fn test_single_entity() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena);

        let entity = Entity::new(0.0, 0.0, 0.0, 1.0);
        tree.push(entity);

        let leaves = tree.get_leaves_with_resolution([0.0; 3], 0.5);
        assert_eq!(
            leaves.len(),
            1,
            "Tree with one entity should return one leaf"
        );
    }

    #[test]
    fn test_entities_at_origin() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 2.0, &arena);

        // Add multiple entities at the same location
        for _ in 0..10 {
            tree.push(Entity::new(0.0, 0.0, 0.0, 1.0));
        }

        let leaves = tree.get_leaves_with_resolution([0.0; 3], 0.5);
        assert_eq!(
            leaves.len(),
            1,
            "this is a test to make sure entities in the same location don't cause infinite recursion"
        );
    }

    #[test]
    fn test_entities_in_different_quadrants() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 2.0, &arena);

        // Place entities in each quadrant
        let positions = [
            [0.5, 0.5, 0.5],   // (+,+,+)
            [-0.5, 0.5, 0.5],  // (-,+,+)
            [0.5, -0.5, 0.5],  // (+,-,+)
            [-0.5, -0.5, 0.5], // (-,-,+)
        ];

        for pos in positions.iter() {
            tree.push(Entity::new(pos[0], pos[1], pos[2], 1.0));
        }

        let leaves = tree.get_leaves_with_resolution([0.0; 3], 0.5);
        assert_eq!(leaves.len(), 4, "All 8 octant entities should be returned");
    }

    #[test]
    fn test_bh_factor_filtering() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 10.0, &arena);

        // Add entities at various distances from origin
        tree.push(Entity::new(0.1, 0.0, 0.0, 1.0)); // very close
        tree.push(Entity::new(5.0, 0.0, 0.0, 1.0)); // far
        tree.push(Entity::new(8.0, 0.0, 0.0, 1.0)); // very far

        // With negative bh_factor, should get all entities
        let all_leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(
            all_leaves.len(),
            3,
            "Negative bh_factor should return all entities"
        );

        // With positive bh_factor, distant entities might be aggregated
        let filtered_leaves = tree.get_leaves_with_resolution([0.0; 3], 1.0);
        assert!(
            filtered_leaves.len() > 0,
            "Should return at least some entities"
        );
    }

    #[test]
    fn test_boundary_conditions() {
        let arena = Bump::new();
        let extent = 1.0;
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], extent, &arena);

        // Entities at the boundaries
        tree.push(Entity::new(extent, 0.0, 0.0, 1.0));
        tree.push(Entity::new(-extent, 0.0, 0.0, 1.0));
        tree.push(Entity::new(0.0, extent, 0.0, 1.0));
        tree.push(Entity::new(0.0, -extent, 0.0, 1.0));

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(leaves.len(), 4, "Boundary entities should be included");
    }

    #[test]
    fn test_query_from_different_locations() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 10.0, &arena);

        // Add entities in a cluster
        for i in 0..20 {
            let offset = i as f64 * 0.1;
            tree.push(Entity::new(offset, 0.0, 0.0, 1.0));
        }

        // Query from origin
        let from_origin = tree.get_leaves_with_resolution([0.0; 3], 0.5);

        // Query from far away
        let from_far = tree.get_leaves_with_resolution([100.0, 0.0, 0.0], 0.5);

        // Results should differ based on observer location
        assert!(from_origin.len() > 0, "Should find entities from origin");
        assert!(from_far.len() > 0, "Should find entities from far location");
    }

    #[test]
    fn test_dense_cluster() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        // Create a dense cluster near origin
        for _ in 0..1000 {
            let x = (rng.random::<f64>() - 0.5) * 0.2;
            let y = (rng.random::<f64>() - 0.5) * 0.2;
            let z = (rng.random::<f64>() - 0.5) * 0.2;
            tree.push(Entity::new(x, y, z, 1.0));
        }

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(
            leaves.len(),
            1000,
            "All entities in dense cluster should be retrievable"
        );
    }

    #[test]
    fn test_sparse_distribution() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 100.0, &arena);
        let mut rng = ChaCha8Rng::seed_from_u64(123);

        // Sparse, evenly distributed entities
        for _ in 0..100 {
            let x = (rng.random::<f64>() - 0.5) * 200.0;
            let y = (rng.random::<f64>() - 0.5) * 200.0;
            let z = (rng.random::<f64>() - 0.5) * 200.0;
            tree.push(Entity::new(x, y, z, 1.0));
        }

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(
            leaves.len(),
            100,
            "All sparse entities should be retrievable"
        );
    }

    #[test]
    fn test_resolution_threshold() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 10.0, &arena);

        // Create entities at known distances
        tree.push(Entity::new(1.0, 0.0, 0.0, 1.0));
        tree.push(Entity::new(5.0, 0.0, 0.0, 1.0));
        tree.push(Entity::new(9.0, 0.0, 0.0, 1.0));

        // Test with increasingly strict resolution
        let loose = tree.get_leaves_with_resolution([0.0; 3], 0.1);
        let medium = tree.get_leaves_with_resolution([0.0; 3], 0.5);
        let strict = tree.get_leaves_with_resolution([0.0; 3], 2.0);

        // More strict resolution should aggregate more
        assert!(
            loose.len() >= medium.len(),
            "Loose resolution should have >= entities than medium"
        );
        assert!(
            medium.len() >= strict.len(),
            "Medium resolution should have >= entities than strict"
        );
    }

    #[test]
    fn test_lots() {
        let arena = Bump::new();
        let extent = 1.0;
        let n = 100_000;
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0 * extent, &arena);

        for _ in 0..n {
            tree.push(Entity::random(&mut rng));
        }

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(
            leaves.len(),
            n,
            "Should retrieve all randomly placed entities"
        );
    }

    #[test]
    fn test_extreme_coordinates() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1000.0, &arena);

        // Test with very large coordinates
        tree.push(Entity::new(999.0, 999.0, 999.0, 1.0));
        tree.push(Entity::new(-999.0, -999.0, -999.0, 1.0));

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(
            leaves.len(),
            2,
            "Extreme coordinates should be handled correctly"
        );
    }

    #[test]
    fn test_zero_extent() {
        let arena = Bump::new();
        // Edge case: what happens with very small extent?
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 0.001, &arena);

        tree.push(Entity::new(0.0, 0.0, 0.0, 1.0));

        let leaves = tree.get_leaves_with_resolution([0.0; 3], -0.1);
        assert_eq!(leaves.len(), 1, "Should handle very small extent");
    }

    #[test]
    fn test_query_outside_bounds() {
        let arena = Bump::new();
        let mut tree: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena);

        tree.push(Entity::new(0.0, 0.0, 0.0, 1.0));

        // Query from location far outside the tree bounds
        let leaves = tree.get_leaves_with_resolution([1000.0, 1000.0, 1000.0], 0.5);
        assert!(
            leaves.len() > 0,
            "Should still find entities when querying from outside"
        );
    }

    #[test]
    fn test_reproducibility() {
        let arena1 = Bump::new();
        let arena2 = Bump::new();
        let mut rng1 = ChaCha8Rng::seed_from_u64(999);
        let mut rng2 = ChaCha8Rng::seed_from_u64(999);

        let mut tree1: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena1);
        let mut tree2: QuadTree<'_, Entity> = QuadTree::new([0.0; 3], 1.0, &arena2);

        for _ in 0..1000 {
            tree1.push(Entity::random(&mut rng1));
            tree2.push(Entity::random(&mut rng2));
        }

        let leaves1 = tree1.get_leaves_with_resolution([0.0; 3], -0.1);
        let leaves2 = tree2.get_leaves_with_resolution([0.0; 3], -0.1);

        assert_eq!(
            leaves1.len(),
            leaves2.len(),
            "Same seed should produce same results"
        );
    }
}
