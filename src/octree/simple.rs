use std::mem;

use log::{info, warn};

type Link<T> = Box<OctreeNode<T>>;

pub trait Entity {
    fn get_mass(&self) -> f32;
    fn get_centre(&self) -> [f32; 3];
    fn fake(centre: [f32; 3], mass: f32) -> Self;
}

#[derive(Default, Debug)]
pub struct Octree<T>
where
    T: Entity,
{
    root: OctreeNode<T>,
}

#[derive(Default, Debug)]
struct OctreeNode<T>
where
    T: Entity,
{
    depth: usize, // 0 is lowest leaf
    centre: [f32; 3],
    extent: f32,
    fake_entity: T,
    stuff: Vec<T>,
    children: [Option<Link<T>>; 8],
    octant_centres: [[f32; 3]; 8],
}

impl<T> Octree<T>
where
    T: Entity + Default,
{
    pub fn new(depth: usize, centre: [f32; 3], extent: f32) -> Self {
        let mut root = OctreeNode::<T>::default();
        root.depth = depth;
        root.extent = extent;
        root.centre = centre;
        Self { root: root }
    }

    pub fn push(&mut self, item: T) {
        self.root.push(item);
    }

    pub fn get(&self, pos: [f32; 3]) -> Vec<&T> {
        self.root.get(pos)
    }

    pub fn get_centres(&self) -> Vec<[f32; 3]> {
        self.root.get_centres()
    }
}

impl<T> OctreeNode<T>
where
    T: Entity + Default,
{
    fn new(depth: usize, centre: [f32; 3], extent: f32) -> Self {
        // todo! put an actual implementation here
        let mut temp = Self::default();
        temp.depth = depth;
        temp.centre = centre;
        temp.extent = extent; // single sided/ analagous to radius

        temp.octant_centres[0] = [
            temp.centre[0] - temp.extent,
            temp.centre[1] - temp.extent,
            temp.centre[2] - temp.extent,
        ];
        temp.octant_centres[1] = [
            temp.centre[0] + temp.extent,
            temp.centre[1] - temp.extent,
            temp.centre[2] - temp.extent,
        ];
        temp.octant_centres[2] = [
            temp.centre[0] - temp.extent,
            temp.centre[1] + temp.extent,
            temp.centre[2] - temp.extent,
        ];
        temp.octant_centres[3] = [
            temp.centre[0] + temp.extent,
            temp.centre[1] + temp.extent,
            temp.centre[2] - temp.extent,
        ];
        temp.octant_centres[4] = [
            temp.centre[0] - temp.extent,
            temp.centre[1] - temp.extent,
            temp.centre[2] + temp.extent,
        ];
        temp.octant_centres[5] = [
            temp.centre[0] + temp.extent,
            temp.centre[1] - temp.extent,
            temp.centre[2] + temp.extent,
        ];
        temp.octant_centres[6] = [
            temp.centre[0] - temp.extent,
            temp.centre[1] + temp.extent,
            temp.centre[2] + temp.extent,
        ];
        temp.octant_centres[7] = [
            temp.centre[0] + temp.extent,
            temp.centre[1] + temp.extent,
            temp.centre[2] + temp.extent,
        ];

        return temp;
    }

    fn push(&mut self, item: T) {
        self.fake_entity = T::fake(self.centre, self.fake_entity.get_mass() + item.get_mass());

        if self.depth == 0 {
            self.stuff.push(item);
        } else {
            // do categorisation of which octree based on entity coordinates
            // and extents.
            let item_pos = item.get_centre();

            let idx = self.get_octant_id(item_pos);

            match self.children[idx].as_mut() {
                Some(node) => {
                    node.as_mut().push(item);
                }
                None => {
                    let mut new_node = Box::new(OctreeNode::<T>::new(
                        self.depth - 1,
                        self.octant_centres[idx],
                        self.extent / 2.0,
                    ));
                    new_node.push(item);
                    self.children[idx] = Some(new_node);
                }
            }
        }
    }

    fn get_octant_id(&self, item_pos: [f32; 3]) -> usize {
        let idx = match item_pos {
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => 0,
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z <= self.centre[2] => 1,
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z <= self.centre[2] => 2,
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z <= self.centre[2] => 3,
            [x, y, z] if x <= self.centre[0] && y <= self.centre[1] && z > self.centre[2] => 4,
            [x, y, z] if x > self.centre[0] && y <= self.centre[1] && z > self.centre[2] => 5,
            [x, y, z] if x <= self.centre[0] && y > self.centre[1] && z > self.centre[2] => 6,
            [x, y, z] if x > self.centre[0] && y > self.centre[1] && z > self.centre[2] => 7,
            _ => unreachable!(),
        };
        idx
    }

    fn get(&self, pos: [f32; 3]) -> Vec<&T> {
        if self.depth == 0 {
            self.stuff.iter().map(|f| f).collect()
        } else {
            let idx = self.get_octant_id(pos);
            warn!("Octant {}", idx);
            let fakes: Vec<&T> = self.children[0..8]
                .iter()
                .enumerate()
                .filter_map(|(i, node)| {
                    if i == idx {
                        None
                    } else {
                        match node {
                            Some(node) => Some(&node.fake_entity),
                            None => None,
                        }
                    }
                })
                .collect();

            match self.children[idx].as_ref() {
                Some(node) => {
                    let mut f = node.get(pos);
                    f.extend(fakes.into_iter());
                    f
                }
                None => fakes,
            }
        }
    }

    fn get_centres(&self) -> Vec<[f32; 3]> {
        if self.depth == 0 {
            return vec![self.centre];
        } else {
            let mut centres = vec![];
            centres.extend(self.octant_centres.iter());
            for i in 0..8 {
                if let Some(node) = self.children[i].as_ref() {
                    centres.extend(node.get_centres().iter());
                }
            }
            return centres;
        }
    }
}

// impl<T> Drop for OctreeNode<T> where T: Entity {
//     fn drop(&mut self) {

//         for i in 0..8 {
//             let mut cur_link = self.children[i].take();
//             for j in 0..8 {
//                 while let Some(mut boxed_node) = cur_link {
//                     cur_link = boxed_node.children[j].take();
//                 }
//             }
//         }
//         // mem::swap(self.children)
//     }
// }

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}
