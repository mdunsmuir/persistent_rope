#![feature(type_ascription)]

#![cfg_attr(feature = "lint", feature(plugin))]
#![cfg_attr(feature = "lint", plugin(clippy))]

use std::slice::Iter;
use std::borrow::Borrow;
use std::ops::Index;
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
    stack: Vec<&'a Link<T>>,
    flat_iter: Iter<'a, T>,
}

impl <T: Clone> Node<T> {

    fn depth(&self) -> usize {
        match *self {
            Concat { depth, .. } => depth,
            Flat { .. } => 0,
        }
    }

    fn len(&self) -> usize {
        match *self {
            Concat { len, .. } => len,
            Flat { ref data } => data.len(),
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
        match *self {
            Flat { ref data } => {
                // TODO: hopefully rust itself will panic on OOB indices here?
                let mut slice = Vec::with_capacity(end - start);
                slice.extend_from_slice(&data[start..end]);
                Rc::new(Flat { data: slice })
            },

            Concat { left_len, left: ref o_left, right: ref o_right, .. } => {
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

        match *self {
            Flat { ref data } => &data[index], // we already checked the bounds
            Concat { left_len, ref left, ref right, .. } => {
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn concat(left: Self, right: Self) -> Self {
        Rope {
            root: Node::concat(&left.root, &right.root),
        }
    }

    pub fn substring(&self, start: usize, end: usize) -> Self {
        if start >= end || end > self.len() {
            panic!("bad substring indices: {}, {}", start, end);
        }

        Rope {
            root: self.root.substring(start, end),
        }
    }

    pub fn iter(&self) -> RopeIter<T> {
        RopeIter::new(&self.root)
    }

}

impl <T: Clone> Index<usize> for Rope<T> {

    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.root.at(index)
    }
}

impl <'a, T: Clone> RopeIter<'a, T> {

    fn new(mut ptr: &'a Link<T>) -> Self {
        let mut stack: Vec<&'a Link<T>> = Vec::with_capacity(ptr.depth());

        loop {
            match *ptr.borrow() {
                Flat { ref data, .. } => {
                    return RopeIter {
                        stack: stack,
                        flat_iter: data.iter(),
                    };
                },

                Concat { ref left, .. } => {
                    stack.push(ptr);
                    ptr = left;
                },
            }
        } // end loop
    }
}

impl <'a, T: Clone> Iterator for RopeIter<'a, T> {

    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {

        // get the next from the iterator on the flat node we're currently
        // pointing at
        match self.flat_iter.next() {
            // if result then just return it
            result@Some(_) => result,

            // otherwise we need to navigate to the next flat node
            None => {
                match self.stack.pop() {

                    // if no nodes are left on the stack we're done
                    None => None,

                    // if a node is on the stack, we already visited its left
                    // children, so go right now and drop the ref to the
                    // popped node
                    Some(rc_ref) => {
                        if let Concat { ref right, .. } = *rc_ref.as_ref() {
                            let mut current = right;

                            // Go left all the way to the next leaf
                            while let Concat { ref left, .. } = *current.as_ref() {
                                self.stack.push(current);
                                current = left;
                            }

                            // load the iterator from this leaf
                            // we finish with the recursive call so that in the
                            // event that this leaf is empty (should not happen
                            // but...) we'll continue on to the next leaf
                            if let Flat { ref data } = *current.as_ref() {
                                self.flat_iter = data.iter();
                                self.next()

                            } else {
                                panic!("should never get here")
                            }

                        } else {
                            panic!("expected only Concat in iter stack")
                        }
                    }
                } // match stack pop
            }
        } // match current iter next
    }
}

#[cfg(test)]
mod tests;
