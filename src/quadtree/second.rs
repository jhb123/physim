use crate::Entity;

type Link<T> = Box<QuadTreeNode<T>>;

#[derive(Default, Debug)]
pub struct QuadTree<T>
where
    T: Entity,
{
    root: QuadTreeNode<T>,
}

#[derive(Default, Debug)]
struct QuadTreeNode<T>
where
    T: Entity,
{
    centre: [f32; 3],
    extent: f32,
    entity: Option<T>,
    children: [Option<Link<T>>; 4],
}

impl<T> QuadTree<T>
where
    T: Entity + Default + Copy + std::fmt::Debug,
{
    pub fn new(centre: [f32; 3], extent: f32) -> Self {
        let root = QuadTreeNode::<T>::new(centre, extent);
        Self { root }
    }

    pub fn push(&mut self, item: T) {
        self.root.push(item, 0);
    }

    pub fn get_leaves(&self) -> Vec<T> {
        self.root.get_leaves()
    }
}

impl<T> QuadTreeNode<T>
where
    T: Entity + Default + Copy + std::fmt::Debug,
{
    fn new(centre: [f32; 3], extent: f32) -> Self {
        // todo! put an actual implementation here
        Self {
            centre,
            extent,
            entity: None,
            ..Default::default()
        }
    }

    fn push(&mut self, item: T, count: usize) {
        match self.entity.as_ref() {
            None => {
                // if there is nothing in the node, put an entity in it.
                self.entity.replace(item);
            }
            Some(current_elem) => {
                
                // this doesn't reliably
                // if current_elem.get_centre().iter().zip(item.get_centre().iter()).all(|(a,b)| f32::abs(a-b) < 0.000001)  {
                //     let fake_elem = T::fake(
                //         item.get_centre(),
                //         current_elem.get_mass() + item.get_mass(),
                //     );
                //     self.entity.replace(fake_elem);
                //     return;
                // }

                // replace the current entity with a new one. take the current one and put it into a child
                let centre_of_mass = current_elem.centre_of_mass(&item);
                let fake_elem = T::fake(centre_of_mass, current_elem.get_mass() + item.get_mass());
                let current_elem = self.entity.replace(fake_elem).unwrap();

                // if this node has no children, the current element is a "real" on and should be added
                // to the corresponding child node
                if self.children.iter().all(|x| x.is_none()){
                    let item_pos = current_elem.get_centre();
                    let idx = self.get_octant_id(item_pos);
                    let mut new_node = Box::new(QuadTreeNode::<T>::new(
                        self.get_octant_id_centre(current_elem.get_centre()),
                        self.extent / 2.0,
                    ));
                    new_node.entity.replace(current_elem);
                    self.children[idx] = Some(new_node);
                }
                
                // insert the new element
                let item_pos = item.get_centre();
                let idx = self.get_octant_id(item_pos);
                match self.children[idx].as_mut() {
                    Some(node) => {
                        node.as_mut().push(item, count + 1);
                    }
                    None => {
                        let mut new_node = Box::new(QuadTreeNode::<T>::new(
                            self.get_octant_id_centre(item.get_centre()),
                            self.extent / 2.0,
                        ));
                        new_node.entity.replace(item);
                        self.children[idx] = Some(new_node);
                    }
                }
           }
        }
    }

    fn get_leaves(&self) -> Vec<T> {
        if self.children.iter().all(|x| x.is_none()){ 
            return vec![self.entity.unwrap()]
        } else {
            let mut elems = vec![];
            for child in self.children.iter() {
                if let Some(child) = child {
                    elems.extend(child.get_leaves())
                }
            }
            return elems;
        }
    }

    fn get_octant_id(&self, item_pos: [f32; 3]) -> usize {
        match item_pos {
            [x, y, _] if x <= self.centre[0] && y <= self.centre[1] => 0,
            [x, y, _] if x > self.centre[0] && y <= self.centre[1] => 1,
            [x, y, _] if x <= self.centre[0] && y > self.centre[1] => 2,
            [x, y, _] if x > self.centre[0] && y > self.centre[1] => 3,
            _ => {
                unreachable!()
            }
        }
    }

    fn get_octant_id_centre(&self, item_pos: [f32; 3]) -> [f32; 3] {
        match item_pos {
            [x, y, _] if x <= self.centre[0] && y <= self.centre[1] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2],
            ],
            [x, y, _] if x > self.centre[0] && y <= self.centre[1] => [
                self.centre[0] + self.extent / 2.0,
                self.centre[1] - self.extent / 2.0,
                self.centre[2],
            ],
            [x, y, _] if x <= self.centre[0] && y > self.centre[1] => [
                self.centre[0] - self.extent / 2.0,
                self.centre[1] + self.extent / 2.0,
                self.centre[2],
            ],
            [x, y, _] if x > self.centre[0] && y > self.centre[1] => [
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

    use super::QuadTree;

    #[test]
    fn test() {}
}
