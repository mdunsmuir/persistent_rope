use std::slice::Iter;
use std::borrow::Borrow;
use std::rc::*;
use std::cmp::{max};

type Link<T> = Rc<Node<T>>;

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
    root: Link<T>,
}

#[derive(Debug)]
pub struct RopeIter<'a, T: 'a> {
    stack: Vec<Link<T>>,
    flat_iter: Iter<'a, T>,
}

impl <T: Clone> Node<T> {

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
            left: left.clone(),
            right: right.clone(),
        })
    }

    /// Create a substring of a Rope
    /// start is inclusive, end is EXclusive.
    fn substring(&self, start: usize, end: usize) -> Rc<Self> {
        match self {
            &Flat { ref data } => {
                // TODO: hopefully rust itself will panic on OOB indices here?
                let mut slice = Vec::with_capacity(end - start);
                slice.extend_from_slice(&data[start..end]);
                Rc::new(Flat { data: slice })
            },

            &Concat { left_len, left: ref o_left, right: ref o_right, .. } => {
                let do_left = start < left_len;
                let do_right = end >= left_len;

                // if the substring straddles this concat node
                if do_left && do_right {
                    let left = o_left.as_ref();
                    let right = o_right.as_ref();

                    let left_sub = left.substring(start, left_len);
                    let right_sub = right.substring(0, end - left_len);

                    Self::concat(&left_sub, &right_sub)
                
                // if we're substringing one side or the other
                } else if do_left {
                    o_left.as_ref().substring(start, end)
                } else if do_right {
                    o_right.as_ref().substring(0, end - left_len)

                // do people do this? I don't know
                } else {
                    panic!("should not have gotten here!")
                }
            }
        }
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

                child.at(new_index)
            },
        }
    }

}

impl <T: Clone> Rope<T> {

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

    pub fn substring(&self, start: usize, end: usize) -> Self {
        if start >= end || end >= self.len() {
            panic!("bad substring indices: {}, {}", start, end);
        }

        Rope {
            root: self.root.substring(start, end),
        }
    }

    pub fn at(&self, index: usize) -> &T {
        self.root.at(index)
    }

    pub fn iter(&self) -> RopeIter<T> {
        RopeIter::new(&self.root)
    }

}

impl <'a, T: Clone> RopeIter<'a, T> {

    fn new(root: &'a Link<T>) -> Self {
        let mut stack: Vec<Link<T>> = Vec::with_capacity(root.depth());
        let mut current: &Link<T> = root;
        
        loop {
            stack.push(current.clone());

            match current.borrow() {
                &Flat { ref data, .. } => {
                    return RopeIter {
                        stack: stack,
                        flat_iter: data.iter(),
                    };
                },

                &Concat { ref left, .. } => {
                    current = left;
                },
            }
        } // end loop
    }
}

impl <'a, T: Clone> Iterator for RopeIter<'a, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let from_inner = self.flat_iter.next();

        match from_inner {
            Some(item) => Some(item),

            // when we're done with the leaf we're currently on
            None => {
                self.stack.pop();

                if let Some(parent_rc) = self.stack.last() {
                    let right = match parent_rc.borrow() {
                        &Concat { ref right, .. } => right.clone(),
                        &Flat { .. } => panic!("did not expect Flat in iter stack"),
                    };

                    // TODO: need tree traversal loop here

                    unimplemented!();
                    //self.stack.push

                // if we're done iterating
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
