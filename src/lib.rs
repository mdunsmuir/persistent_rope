//!
//! An implementation of a reference-counted persistent rope data structure.
//! It is intended to allow (relatively) efficient storage of long sequences of
//! values, and (relatively) efficient create/read/concat/slice operations on 
//! said sequences. Its features are motivated by the needs of a hypothetical
//! text editor: persistence gives you easy undo functionality and allows you 
//! to operate on the buffer concurrently e.g. saving a copy while the user
//! continues to edit, without having to copy the entire buffer in memory.
//! Markers allow sparse annotation of the buffer with useful information
//! e.g. the locations of line breaks.
//!
//! The implementation is based on a paper called "Ropes: an Alternative to 
//! Strings" (Boehm, Atkinson, and Plass 1995).
//!
//! # Usage
//!
//! Create a flat `Rope` from a slice:
//!
//! ```
//! use persistent_rope::Rope;
//! let rope: Rope<usize> = Rope::new(&vec![1, 2, 3]);
//! ```
//!
//! A `Rope` is immutable and operations that "change" `Rope`s actually create
//! new "copies" of them thanks to persistence. Immutability means that
//! can use reference-counted pointers to avoid copying the entire structure
//! whenever we change something:
//!
//! ```
//! use persistent_rope::Rope;
//! let rope: Rope<usize> = Rope::new(&vec![1, 2, 3]);
//!
//! let concatted = Rope::concat(&rope, &Rope::concat(&rope, &rope));
//! let concatted_as_vec: Vec<usize> = concatted.iter().cloned().collect();
//! assert_eq!(vec![1, 2, 3, 1, 2, 3, 1, 2, 3], concatted_as_vec);
//!
//! let concatted_sliced_as_vec: Vec<usize> = concatted.slice(2, 6)
//!                                                    .iter()
//!                                                    .cloned()
//!                                                    .collect();
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
//! # TODO
//!
//! * Loading data could still be more space and time efficient, possibly
//!   with the addition of lazy-loading nodes or nodes that point into a
//!   shared slice.
//!
//! # Disclaimer
//!
//! This code is in a very rough state. I intend to finish, polish, benchmark,
//! and optimize it, but as of now I've done none of those things and I make
//! no guarantees about it, not even that its essential design is capable of
//! acceptable performance.
//!

#![cfg_attr(feature = "lint", feature(plugin))]
#![cfg_attr(feature = "lint", plugin(clippy))]

use std::slice::Iter;
use std::borrow::Borrow;
use std::ops::Index;
use std::rc::*;
use std::cmp::{max};

use std::hash::Hash;
use std::collections::HashMap;
use std::collections::BTreeSet;

type Link<T, M> = Rc<Node<T, M>>;
//type Markers<M> = BTreeMap<usize, HashSet<M>>;
type Markers<M> = HashMap<M, BTreeSet<usize>>;

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
        markers: Markers<M>,
    },
}

use Node::*;

pub struct Rope<T, M = ()> {
    root: Link<T, M>,
}

pub struct Values<'a, T: 'a, M: 'a + Eq + Hash> {
    stack: Vec<&'a Link<T, M>>,
    flat_iter: Iter<'a, T>,
}

/// Used in the creation of new `Rope`s
pub struct Chunk<T, M> {
    data: Vec<T>,
    markers: Markers<M>,
}

impl<T: Clone, M: Eq + Hash + Copy> Chunk<T, M> {

    pub fn with_capacity(capacity: usize) -> Self {
        Chunk {
            data: Vec::with_capacity(capacity),
            markers: HashMap::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        self.data.extend_from_slice(slice);
    }

    pub fn mark_at(&mut self, marker: M, at: usize) {
        if at >= self.data.len() {
            panic!("attempted to mark outside data range");
        } else {
            self.markers.entry(marker)
                        .or_insert(BTreeSet::new())
                        .insert(at);
        }
    }
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

                for (marker, marker_indices) in markers.iter() {
                    for _ in marker_indices {
                        *counts.entry(*marker).or_insert(0) += 1;
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
            depth: max(left.depth(), right.depth()) + 1,
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

                let mut new_markers = HashMap::new();

                for (&marker, ref indices) in markers.iter() {
                    let sliced_markers: BTreeSet<usize> =
                        indices.iter()
                               .cloned()
                               .filter(|&i| i >= start && i < end)
                               .collect();

                    if !sliced_markers.is_empty() {
                        new_markers.insert(marker, sliced_markers);
                    }
                }

                Rc::new(Flat { data: slice, markers: new_markers })
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

    /// Find the index in the rope which has been marked with the `n`th
    /// instance of `marker`. Useful for e.g. finding the index of the `n`th
    /// newline.
    fn index_for_nth_marker(&self, marker: M, n: usize) -> Option<usize> {
        match *self {
            Flat { ref markers, .. } => {
                match markers.get(&marker) {
                    None => None,
                    Some(indices) => indices.iter().nth(n).map(|&i| i)
                }
            },

            Concat { ref markers, ref left, ref right, left_len, .. } => {
                match markers.get(&marker) {
                    None => None,
                    Some(&(left_count, count)) => {
                        if n < left_count {
                            left.index_for_nth_marker(marker, n)
                        } else if n < count {
                            right.index_for_nth_marker(marker, n - left_count)
                                 .map(|i| left_len + i)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

}

impl<T: Clone, M: Eq + Hash + Copy> Rope<T, M> {

    pub fn new(data: &[T]) -> Self {
        let mut data_vec = Vec::with_capacity(data.len());
        data_vec.extend_from_slice(&data);

        Rope { root: Rc::new( Flat {
            data: data_vec,
            markers: HashMap::new(),
        })}
    }

    pub fn from_chunk(chunk: Chunk<T, M>) -> Self {
        Rope { root: Rc::new(Flat {
            data: chunk.data,
            markers: chunk.markers,
        })}
    }

    /// The nodes in the rope are all immutable, so creating a new rope is
    /// most efficient if we create all the leaf nodes first so we don't
    /// have to do any traversal and reallocation.
    ///
    /// The closure supplied to this method is expected to return
    /// `Ok(Some(chunk))` some number of times, terminating the loading
    /// procedure either by returning `Ok(None)`, which indicates end of input
    /// and triggers the assembly of the rope, or `Err(some_error)` which
    /// indicates e.g. an IO or decoding error. In the latter case, that error
    /// is returned from this method call as `Err(some_error)`.
    ///
    /// We don't put any restrictions on the size of chunks, but clients are
    /// encouraged to use the `Chunk::with_capacity(capacity)` initializer,
    /// where `capacity` is the maximum number of items expected per-chunk,
    /// to avoid continuous reallocations.
    pub fn from_chunks<F, E>(mut loader: F) -> Result<Self, E>
        where F: FnMut() -> Result<Option<Chunk<T, M>>, E> {

        let mut stack = Vec::new();

        'outer: loop {
            match loader() {
                Err(e) => return Err(e),
                Ok(None) => break 'outer,

                Ok(Some(chunk)) => {
                    stack.push(Self::from_chunk(chunk));

                    while stack.len() > 1 &&
                        stack[stack.len() - 1].depth() == stack[stack.len() - 2].depth() {

                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                        stack.push(Self::concat(&left, &right));
                    }
                } // end match OK
            }
        } // end 'outer

        let init = stack.pop().unwrap();
        let rope = stack.into_iter()
                        .rev()
                        .fold(init, |right, left| Rope::concat(&left, &right));

        Ok(rope)
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn depth(&self) -> usize {
        self.root.depth()
    }

    pub fn marker_counts(&self) -> HashMap<M, usize> {
        self.root.marker_counts()
    }

    pub fn marker_count(&self, marker: M) -> usize {
        match self.marker_counts().get(&marker) {
            None => 0,
            Some(&n) => n,
        }
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

    pub fn index_for_nth_marker(&self, marker: M, n: usize) -> Option<usize> {
        self.root.index_for_nth_marker(marker, n)
    }

    pub fn iter(&self) -> Values<T, M> {
        Values::new(&self.root)
    }

}

impl<T: Clone, M: Eq + Hash + Copy> Index<usize> for Rope<T, M> {

    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.root.at(index)
    }
}

impl<'a, T: Clone, M: Eq + Hash + Copy> Values<'a, T, M> {

    fn new(mut ptr: &'a Link<T, M>) -> Self {
        let mut stack: Vec<&'a Link<T, M>> = Vec::with_capacity(ptr.depth());

        loop {
            match *ptr.borrow() {
                Flat { ref data, .. } => {
                    return Values {
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

impl<'a, T: Clone, M: Eq + Hash + Copy> Iterator for Values<'a, T, M> {

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
    type IntoIter = Values<'a, T, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests;
