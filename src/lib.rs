//!
//! An implementation of a reference-counted persistent rope data structure.
//! It is intended to allow (relatively) efficient storage of long sequences of
//! values, and efficient concat/slice operations on said sequences.
//!
//! The implementation is based on a paper called "Ropes: an Alternative to 
//! Strings" (Boehm, Atkinson, and Plass 1995).
//!
//! # Usage
//!
//! Create a `Rope` by consuming a `Vec`:
//!
//! ```
//! use persistent_rope::Rope;
//! let rope: Rope<usize> = Rope::new(vec![1, 2, 3]);
//! ```
//!
//! A `Rope` is immutable and operations that "change" `Rope`s actually create
//! new "copies" of them thanks to persistence. Immutability means that
//! can use reference-counted pointers to avoid copying the entire structure
//! whenever we change something:
//!
//! ```
//! use persistent_rope::Rope;
//! let rope: Rope<usize> = Rope::new(vec![1, 2, 3]);
//!
//! let concatted = Rope::concat(&rope, &Rope::concat(&rope, &rope));
//! let concatted_as_vec: Vec<usize> = concatted.iter().cloned().collect();
//! assert_eq!(vec![1, 2, 3, 1, 2, 3, 1, 2, 3], concatted_as_vec);
//!
//! let concatted_sliced_as_vec: Vec<usize> = concatted.slice(2, 6)
//!                                                         .iter()
//!                                                         .cloned()
//!                                                         .collect();
//! assert_eq!(vec![3, 1, 2, 3], concatted_sliced_as_vec);
//!
//! ```
//!
//! ## Markers
//!
//! Markers (denoted by type parameter `M`) allow positions in the sequence to
//! be stored (sparsely) to enable operations like e.g. "give me the index of
//! the 43rd line break in my text buffer" or "how many open parenthesis
//! characters appear in my text buffer".
//!
//! # Disclaimer
//!
//! This code is in a very rough state. I intend to finish, polish, benchmark,
//! and optimize it, but as of now I've done none of those things and I make
//! no guarantees about it, not even that its essential design is capable of
//! acceptable performance.
//!

#![feature(type_ascription)]
#![feature(collections_bound)]
#![feature(btree_range)]

#![cfg_attr(feature = "lint", feature(plugin))]
#![cfg_attr(feature = "lint", plugin(clippy))]

use std::slice::Iter;
use std::borrow::Borrow;
use std::ops::Index;
use std::rc::*;
use std::cmp::{max};

use std::hash::Hash;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::BTreeMap;
use std::collections::Bound::*;

type Link<T, M> = Rc<Node<T, M>>;

enum Node<T, M> {
    Concat {
        depth: usize,
        left_len: usize,
        markers: HashMap<M, (usize, usize)>,
        len: usize,
        left: Link<T, M>,
        right: Link<T, M>,
    },

    Flat {
        data: Vec<T>,
        markers: BTreeMap<usize, HashSet<M>>,
    },
}

use Node::*;

pub struct Rope<T, M = ()> {
    root: Link<T, M>,
}

pub struct RopeIter<'a, T: 'a, M: 'a + Eq + Hash> {
    stack: Vec<&'a Link<T, M>>,
    flat_iter: Iter<'a, T>,
}

impl<T: Clone, M: Eq + Hash + Copy> Node<T, M> {

    fn depth(&self) -> usize {
        match *self {
            Concat { depth, .. } => depth,
            Flat { .. } => 0,
        }
    }

    fn len(&self) -> usize {
        match *self {
            Concat { len, .. } => len,
            Flat { ref data, .. } => data.len(),
        }
    }

    fn marker_counts(&self) -> HashMap<M, usize> {
        match *self {
            Concat { ref markers, .. } => {
                markers.iter()
                       .map(|(&marker, &(_, count))| (marker, count) )
                       .collect()
            },

            Flat { ref markers, .. } => {
                let mut counts = HashMap::new();

                for markers_at in markers.values() {
                    for &marker in markers_at {
                        *counts.entry(marker).or_insert(0) += 1;
                    }
                }

                counts
            }
        }
    }

    // TODO: Optimize for concatenating short subtrees -> Flat
    fn concat(left: &Rc<Self>, right: &Rc<Self>) -> Rc<Self> {
        let mut counts: HashMap<M, (usize, usize)> =
            left.marker_counts()
                .iter()
                .map(|(&marker, &count)| (marker, (count, count)) )
                .collect();

        for (&marker, &count) in &right.marker_counts() {
            counts.entry(marker).or_insert((0, 0)).1 += count;
        }

        Rc::new(Concat {
            depth: max(left.depth(), right.depth()),
            left_len: left.len(),
            markers: counts,
            len: left.len() + right.len(),
            left: left.clone(),
            right: right.clone(),
        })
    }

    fn slice(&self, start: usize, end: usize) -> Rc<Self> {
        match *self {
            Flat { ref data, ref markers } => {
                // TODO: hopefully rust itself will panic on OOB indices here?
                let mut slice = Vec::with_capacity(end - start);
                slice.extend_from_slice(&data[start..end]);

                let sliced_markers =
                    markers.range(Included(&start),
                                  Excluded(&end))
                           .map(|(&index, set)| (index, set.clone()))
                           .collect();

                Rc::new(Flat { data: slice, markers: sliced_markers })
            },

            Concat { left_len, left: ref o_left, right: ref o_right, .. } => {
                let do_left = start < left_len;
                let do_right = end >= left_len;

                // if the slice straddles this concat node
                if do_left && do_right {
                    let left = o_left.as_ref();
                    let right = o_right.as_ref();

                    let left_sub = left.slice(start, left_len);
                    let right_sub = right.slice(0, end - left_len);

                    Self::concat(&left_sub, &right_sub)
                
                // if we're sliceing one side or the other
                } else if do_left {
                    o_left.as_ref().slice(start, end)
                } else if do_right {
                    o_right.as_ref().slice(0, end - left_len)

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
            Flat { ref data, .. } => &data[index], // we already checked the bounds
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

impl<T: Clone, M: Eq + Hash + Copy> Rope<T, M> {

    pub fn new(data: Vec<T>) -> Self {
        Rope {
            root: Rc::new( Flat { data: data, markers: BTreeMap::new() } ),
        }
    }

    pub fn with_markers(data: Vec<(T, Option<HashSet<M>>)>) -> Self {
        let mut marker_map = BTreeMap::new();

        let values = data.into_iter()
                         .enumerate()
                         .map(|(i, (value, o_markers))| {

            if let Some(markers) = o_markers {
                marker_map.insert(i, markers);
            }

            value
        }).collect();

        Rope { root: Rc::new(Flat {
            data: values,
            markers: marker_map,
        })}
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn marker_counts(&self) -> HashMap<M, usize> {
        self.root.marker_counts()
    }

    pub fn concat(left: &Self, right: &Self) -> Self {
        Rope {
            root: Node::concat(&left.root, &right.root),
        }
    }

    /// `start` is inclusive, `end` is EXclusive.
    pub fn slice(&self, start: usize, end: usize) -> Self {
        if start >= end || end > self.len() {
            panic!("bad slice indices: {}, {}", start, end);
        }

        Rope {
            root: self.root.slice(start, end),
        }
    }

    pub fn iter(&self) -> RopeIter<T, M> {
        RopeIter::new(&self.root)
    }

}

impl<T: Clone, M: Eq + Hash + Copy> Index<usize> for Rope<T, M> {

    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.root.at(index)
    }
}

impl<'a, T: Clone, M: Eq + Hash + Copy> RopeIter<'a, T, M> {

    fn new(mut ptr: &'a Link<T, M>) -> Self {
        let mut stack: Vec<&'a Link<T, M>> = Vec::with_capacity(ptr.depth());

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

impl<'a, T: Clone, M: Eq + Hash + Copy> Iterator for RopeIter<'a, T, M> {

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
                            if let Flat { ref data, .. } = *current.as_ref() {
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

impl<'a, T: Clone, M: Eq + Hash + Copy> IntoIterator for &'a Rope<T, M> {

    type Item = &'a T;
    type IntoIter = RopeIter<'a, T, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests;
