use bumpalo::{Bump, boxed};

use crate::Star;

type Link<'a, T> = boxed::Box<'a, OctreeNode<'a, T>>;

#[derive(Debug)]
pub struct Octree<'a, T>
where
    T: Star,
{
    root: OctreeNode<'a, T>,
    arena: &'a Bump,
}

#[derive(Default, Debug)]
struct OctreeNode<'a, T>
where
    T: Star,
{
    centre: [f64; 3],
    extent: f64,
    entity: Option<T>,
    children: [Option<Link<'a, T>>; 8],
}

impl<'a, T> Octree<'a, T>
where
    T: Star + Default + Copy,
{
    pub fn new(centre: [f64; 3], extent: f64, arena: &'a Bump) -> Self {
        let root = OctreeNode::<T>::new(centre, extent);
        Self { root, arena }
    }

    pub fn push(&mut self, item: T) {
        self.root.push(item, 0, self.arena);
    }

    #[allow(dead_code)]
    pub fn get_leaves(&self) -> Vec<T> {
        self.root.get_leaves()
    }

    pub fn get_leaves_with_resolution(&self, location: [f64; 3], bh_factor: f64) -> Vec<T> {
        self.root.get_leaves_with_resolution(location, bh_factor)
    }
}

impl<'a, T> OctreeNode<'a, T>
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
                        .all(|(a, b)| f64::abs(a - b) < 0.001)
                {
                    let fake_elem =
                        T::fake(item.get_centre(), current_elem.get_mass() + item.get_mass());
                    self.entity.replace(fake_elem);
                    return;
                }

                // replace the current entity with a new one. take the current one and put it into a child
                let centre_of_mass = current_elem.centre_of_mass(&item);
                let fake_elem = T::fake(centre_of_mass, current_elem.get_mass() + item.get_mass());
                let current_elem = self.entity.replace(fake_elem).unwrap();

                // if this node has no children, the current element is a "real" on and should be added
                // to the corresponding child node
                if self.children.iter().all(|x| x.is_none()) {
                    let item_pos = current_elem.get_centre();
                    let idx = self.get_octant_id(item_pos);
                    // let mut new_node = Box::new_in(OctreeNode::<T>, arena);
                    let mut new_node = boxed::Box::new_in(
                        OctreeNode::<T>::new(
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
                            OctreeNode::<T>::new(
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

    fn get_leaves(&self) -> Vec<T> {
        if self.children.iter().all(|x| x.is_none()) {
            vec![self.entity.unwrap()]
        } else {
            let mut elems = vec![];
            for child in self.children.iter().flatten() {
                elems.extend(child.get_leaves())
            }
            elems
        }
    }

    fn get_leaves_with_resolution(&self, location: [f64; 3], bh_factor: f64) -> Vec<T> {
        if let Some(e) = self.entity {
            let r = ((location[0] - self.centre[0]).powi(2)
                + (location[1] - self.centre[1]).powi(2)
                + (location[2] - self.centre[2]).powi(2))
            .sqrt();
            if self.extent / r < bh_factor {
                return vec![e];
            }
        }

        if self.children.iter().all(|x| x.is_none()) {
            vec![self.entity.unwrap()]
        } else {
            let mut elems = vec![];
            for child in self.children.iter().flatten() {
                elems.extend(child.get_leaves_with_resolution(location, bh_factor))
            }
            elems
        }
    }

    fn get_octant_id(&self, item_pos: [f64; 3]) -> usize {
        match item_pos {
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => 0,
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => 1,
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z <= self.centre[2] => 2,
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z <= self.centre[2] => 3,
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z > self.centre[2] => 4,
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z > self.centre[2] => 5,
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z > self.centre[2] => 6,
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z > self.centre[2] => 7,
            _ => {
                unreachable!()
            }
        }
    }

    fn get_octant_id_centre(&self, item_pos: [f64; 3]) -> [f64; 3] {
        match item_pos {
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2] - self.extent / 2.0,
            ],
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2] - self.extent / 2.0,
            ],
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z <= self.centre[2] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2] - self.extent / 2.0,
            ],
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z <= self.centre[2] => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2] - self.extent / 2.0,
            ],
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z > self.centre[2] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2] + self.extent / 2.0,
            ],
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z > self.centre[2] => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2] + self.extent / 2.0,
            ],
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z > self.centre[2] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2] + self.extent / 2.0,
            ],
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z > self.centre[2] => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2] + self.extent / 2.0,
            ],
            _ => {
                unreachable!()
            }
        }
    }
}
