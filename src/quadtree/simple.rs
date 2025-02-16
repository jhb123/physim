use crate::Entity;

type Link<T> = Box<QuadreeNode<T>>;

#[derive(Default, Debug)]
pub struct Quadree<T>
where
    T: Entity,
{
    root: QuadreeNode<T>,
}

#[derive(Default, Debug)]
struct QuadreeNode<T>
where
    T: Entity,
{
    depth: usize, // 0 is lowest leaf
    centre: [f32; 3],
    extent: f32,
    fake_entity: T,
    stuff: Vec<T>,
    children: [Option<Link<T>>; 4],
    quad_centres: [[f32; 3]; 4],
}

impl<T> Quadree<T>
where
    T: Entity + Default,
{
    pub fn new(depth: usize, centre: [f32; 3], extent: f32) -> Self {
        let root = QuadreeNode::<T>::new(depth, centre, extent);
        Self { root }
    }

    pub fn push(&mut self, item: T) {
        self.root.push(item);
    }

    pub fn get(&self, pos: [f32; 3]) -> Vec<&T> {
        self.root.get(pos)
    }

    pub fn get_real(&self, pos: [f32; 3]) -> Vec<&T> {
        self.root.get_real(pos)
    }

    pub fn get_fakes(&self, pos: [f32; 3]) -> Vec<&T> {
        self.root.get_fakes(pos)
    }

    pub fn get_centres(&self) -> Vec<[f32; 3]> {
        self.root.get_centres()
    }

    pub fn get_leaf_centres(&self) -> Vec<[f32; 3]> {
        self.root.get_leaf_centres()
    }

}

impl<T> QuadreeNode<T>
where
    T: Entity + Default,
{
    fn new(depth: usize, centre: [f32; 3], extent: f32) -> Self {
        // todo! put an actual implementation here
        let mut node = Self {
            depth,
            centre,
            extent,
            ..Default::default()
        };
        node.quad_centres[0] = [
            node.centre[0] - node.extent,
            node.centre[1] - node.extent,
            node.centre[2],
        ];
        node.quad_centres[1] = [
            node.centre[0] + node.extent,
            node.centre[1] - node.extent,
            node.centre[2],
        ];
        node.quad_centres[2] = [
            node.centre[0] - node.extent,
            node.centre[1] + node.extent,
            node.centre[2],
        ];
        node.quad_centres[3] = [
            node.centre[0] + node.extent,
            node.centre[1] + node.extent,
            node.centre[2],
        ];
        node
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
                    let mut new_node = Box::new(QuadreeNode::<T>::new(
                        self.depth - 1,
                        self.quad_centres[idx],
                        self.extent / 2.0,
                    ));
                    new_node.push(item);
                    self.children[idx] = Some(new_node);
                }
            }
        }
    }

    fn get_octant_id(&self, item_pos: [f32; 3]) -> usize {
        match item_pos {
            [x, y, _] if x <= self.centre[0] && y <= self.centre[1] => 0,
            [x, y, _] if x > self.centre[0] && y <= self.centre[1] => 1,
            [x, y, _] if x <= self.centre[0] && y > self.centre[1] => 2,
            [x, y, _] if x > self.centre[0] && y > self.centre[1] => 3,
            _ => unreachable!(),
        }
    }

    fn get_real(&self, pos: [f32; 3]) -> Vec<&T> {
        if self.depth == 0 {
            self.stuff.iter().collect()
        } else {
            let idx = self.get_octant_id(pos);
            match self.children[idx].as_ref() {
                Some(node) => {
                    node.get_real(pos)
                }
                None => vec![],
            }
        }
    }

    fn get(&self, pos: [f32; 3]) -> Vec<&T> {
        if self.depth == 0 {
            self.stuff.iter().collect()
        } else {
            let idx = self.get_octant_id(pos);
            let fakes: Vec<&T> = self.children[0..4]
                .iter()
                .enumerate()
                .filter_map(|(i, node)| {
                    if i == idx {
                        None
                    } else {
                        node.as_ref().map(|node| &node.fake_entity)
                    }
                })
                .collect();

            match self.children[idx].as_ref() {
                Some(node) => {
                    let mut f = node.get(pos);
                    f.extend(fakes);
                    f
                }
                None => fakes,
            }
        }
    }

    fn get_centres(&self) -> Vec<[f32; 3]> {
        let mut centres = vec![self.centre];
        if self.depth == 0 {
            centres
        } else {
            centres.extend(self.quad_centres.iter());
            for i in 0..4 {
                if let Some(node) = self.children[i].as_ref() {
                    centres.extend(node.get_centres().iter());
                }
            }
            centres
        }
    }

    fn get_leaf_centres(&self) -> Vec<[f32; 3]> {
        if self.depth == 0 {
            vec![self.centre]
        } else {
            let mut centres = vec![];
            for i in 0..4 {
                if let Some(node) = self.children[i].as_ref() {
                    centres.extend(node.get_leaf_centres().iter());
                }
            }
            centres
        }
    }

    fn get_fakes(&self, pos: [f32; 3]) -> Vec<&T> {
        if self.depth == 0 {
           vec![]
        } else {
            let idx = self.get_octant_id(pos);
            let mut fakes: Vec<&T> = vec![];
            self.children[0..4]
                .iter()
                .enumerate()
                .for_each(|(i, node)| {
                    if i == idx {
                        node.as_ref().map(|node| fakes.extend(node.get_fakes(pos)));
                    } else {
                        node.as_ref().map(|node| fakes.push(&node.fake_entity));
                    }
                });            
        
            fakes
        }
    }
}

// impl<T> Drop for QuadreeNode<T> where T: Entity {
//     fn drop(&mut self) {

//         for i in 0..4 {
//             let mut cur_link = self.children[i].take();
//             for j in 0..4 {
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
    use crate::stars::Star;

    use super::Quadree;


    #[test]
    fn test_leaf_centres() {
        let mut stars = Vec::with_capacity(10000);
        for _ in 0..10_000 {
            stars.push(Star::random());
        }
        let mut tree = Quadree::new(1, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_leaf_centres().len(), 4);

        let mut tree = Quadree::new(2, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_leaf_centres().len(), 16);


        let mut tree = Quadree::new(4, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_leaf_centres().len(), 256);
    }

    #[test]
    fn test_get_fakes() {
        let mut stars = Vec::with_capacity(10000);
        for _ in 0..10_000 {
            stars.push(Star::random());
        }
        let mut tree = Quadree::new(1, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_fakes([0.0, 0.0, 0.0]).len(), 3);

        let mut tree = Quadree::new(2, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_fakes([0.0, 0.0, 0.0]).len(), 3*2);

        let mut tree = Quadree::new(5, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }
        assert_eq!(tree.get_fakes([0.0, 0.0, 0.0]).len(), 3*5);
    }

    #[test]
    fn test_get_real() {
        let len = 10_000;
        let mut stars = Vec::with_capacity(len);
        for _ in 0..len {
            stars.push(Star::random());
        }
        let mut tree = Quadree::new(5, [0.0, 0.0, 0.0], 0.5);
        for star in stars.iter() {
            tree.push(*star);
        }

        let centres = tree.get_leaf_centres();
        // println!("{:?}",tree.get_only_real([0.2,0.2,0.2]));
        let mut remade_stars: Vec<&Star> = vec![];
        for centre in centres {
            remade_stars.extend(tree.get_real(centre));
        }
        assert_eq!(remade_stars.len(), len)
    }

}
