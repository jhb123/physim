use std::rc::{Rc, Weak};
use std::cell::RefCell;

#[derive(Debug)]
pub struct Tree {
    pub root: Rc<RefCell<Node>>
}

impl Tree {

    pub fn new(elem: i32) -> Self {
        Self{root: Rc::new(RefCell::new(Node::new(elem))) }
    }

    pub fn push(&mut self, elem: i32) {
        Node::push(&self.root, elem);
    }

    pub fn get_left(&self) -> Option<Rc<RefCell<Node>>> {
        Node::get_left(self.root.clone())
    }

    pub fn get_leaves(&self) {
        Node::get_leaves(self.root.clone());
    }

}


#[derive(Debug)]
pub struct Node {
    elem: i32,
    parent: Weak<RefCell<Node>>,
    left: Option<Rc<RefCell<Node>>>,
    right: Option<Rc<RefCell<Node>>>,
}

impl Node {

    pub fn new(elem: i32) -> Self {
        Self {
            elem,
            parent: Weak::new(),
            left: None,
            right: None
        }

    }

    pub fn get_parent(&self) -> Weak<RefCell<Node>>{
        self.parent.clone()
    }

    pub fn push(self_rc: &Rc<RefCell<Self>>, elem: i32) {
        let mut self_borrow = self_rc.borrow_mut();

        if elem < self_borrow.elem {
            match self_borrow.left.as_mut() {
                Some(node) => {
                    Node::push(node, elem);
                },
                None => {
                    let mut n = Node::new(elem);
                    n.parent = Rc::downgrade(self_rc);// get current self_rc
                    self_borrow.left = Some(Rc::new(RefCell::new(n)))
                },
            }
        } else {
            match self_borrow.right.as_mut() {
                Some(node) => {
                    Node::push(node, elem);
                },
                None => {
                    let mut n = Node::new(elem);
                    n.parent = Rc::downgrade(self_rc);// get current self_rc
                    self_borrow.right = Some(Rc::new(RefCell::new(n)))
                },
            }
        }
    }

    pub fn get_left(self_rc: Rc<RefCell<Self>>)-> Option<Rc<RefCell<Node>>> {
        let left_child = self_rc.borrow().left.clone();
        match left_child {
            Some(n) => Node::get_left(n),
            None => None,
        }
    }

    pub fn get_right(self_rc: Rc<RefCell<Self>>)-> Option<Rc<RefCell<Node>>> {
        let right_child = self_rc.borrow().right.clone();
        match right_child {
            Some(n) => Node::get_right(n),
            None => None,
        }
    }

    pub fn get_leaves(self_rc: Rc<RefCell<Self>>) {
        if self_rc.borrow().left.is_none() && self_rc.borrow().right.is_none() {
            println!("{}", self_rc.borrow().elem)
        } else {
            if self_rc.borrow().left.is_some() {
                Node::get_leaves(self_rc.borrow().left.as_ref().unwrap().clone());
            }
            if self_rc.borrow().right.is_some() {
                Node::get_leaves(self_rc.borrow().right.as_ref().unwrap().clone());
            }

        }
    }
}


