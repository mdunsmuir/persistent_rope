use std::rc::*;
use std::cmp::{max};

type Link<T> = Option<Rc<Node<T>>>;

#[derive(Debug)]
enum Node<T> {
    Concat {
        depth: usize,
        left_len: usize,
        len: usize,
        left: Link<T>,
        right: Link<T>,
    },
    Flat {
        data: Vec<T>,
    },
}

use Node::*;

#[derive(Debug)]
pub struct Rope<T> {
    root: Rc<Node<T>>,
}

impl <T> Node<T> {

    fn depth(&self) -> usize {
        match self {
            &Concat { depth, .. } => depth,
            &Flat { .. } => 0,
        }
    }

    /*
    fn left_len(&self) -> usize {
        match self {
            &Concat { left_len, .. } => left_len,
            &Flat { ref data } => data.len(),
        }
    }
    */

    fn len(&self) -> usize {
        match self {
            &Concat { len, .. } => len,
            &Flat { ref data } => data.len(),
        }
    }

    // TODO: Optimize for concatenating short subtrees -> Flat
    fn concat(left: &Rc<Self>, right: &Rc<Self>) -> Rc<Self> {
        Rc::new(Concat {
            depth: max(left.depth(), right.depth()),
            left_len: left.len(),
            len: left.len() + right.len(),
            left: Some(left.clone()),
            right: Some(right.clone()),
        })
    }

    fn at(&self, index: usize) -> &T {
        if index >= self.len() {
            panic!("index exceeds bounds (length {:?}, index {:?})", self.len(), index)
        }

        match self {
            &Flat { ref data } => &data[index], // we already checked the bounds
            &Concat { left_len, ref left, ref right, .. } => {
                let (child, new_index) = 
                    if index < left_len {
                        (left, index)
                    } else {
                        (right, index - left_len)
                    };

                match child {
                    &None => panic!("expected a child, but none found"),
                    &Some(ref child) => child.at(new_index),
                }
            },
        }
    }

}

impl <T> Rope<T> {

    pub fn new(data: Vec<T>) -> Self {
        Rope {
            root: Rc::new( Flat { data: data } ),
        }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn concat(left: Self, right: Self) -> Self {
        Rope {
            root: Node::concat(&left.root, &right.root),
        }
    }

    pub fn at(&self, index: usize) -> &T {
        self.root.at(index)
    }

}

#[cfg(test)]
mod tests;
