use std::rc::*;

type Rrcc<T> = Rc<RefCell<T>>;
type Wrc<T> = Weak<RefCell<T>>;

type DownLink<T> = Option<Rrcc<Node<T>>>;
type UpLink<T> = Option<Wrc<Node<T>>>;

#[derive(Debug)]
enum Node<T> {
    Stem {
        weight: usize,
        parent: UpLink<T>,
        left: DownLink<T>,
        right: DownLink<T>,
    },
    Leaf {
        fragment: Vec<T>,
        parent: UpLink<T>,
    },
}

#[derive(Debug)]
pub struct Rope<T> {
    root: DownLink<T>,
    fragment_size: usize,
}

pub struct Cursor<'a, T: 'a> {
    ptr: Rrcc<Node<T>>,
    at: usize,
    fragment_start: usize,
    phantom_data: PhantomData<&'a T>,
}

use self::Node::*;

macro_rules! child_accessor {
    ($child_name:ident) => (
        fn $child_name(&self) -> Option<Ref<Self>> {
            match self {
                &Leaf { .. } => None,
                &Stem { $child_name: ref child_ref, .. } =>
                    child_ref.as_ref().map(|rrcc| {
                        rrcc.as_ref().borrow()
                    }),
            }
        }
    )
}


impl <T> Node<T> {

    /// The length of the sequence stored under this Node
    fn len(&self) -> usize {
        match self {
            &Leaf { ref fragment, .. } => fragment.len(),

            &Stem { weight, ref right, .. } => {
                weight + match right {
                    &Some(ref node) => node.borrow().len(),
                    &None => 0,
                }
            },
        }
    }

    child_accessor!(left);
    child_accessor!(right);

}

impl <T: Copy> Rope<T> {

    pub fn from_slice(source: &[T], fragment_size: usize) -> Self {
        let n_fragments = source.len() / fragment_size + 1;

        let mut dq_a = VecDeque::with_capacity(n_fragments);
        let mut dq_b = VecDeque::with_capacity(n_fragments / 2 + 1);
        let mut dq_swap;

        for chunk in source.chunks(fragment_size) {
            let mut fragment = Vec::with_capacity(fragment_size);
            for value in chunk {
                fragment.push(*value);
            }

            //fragment.copy_from_slice(chunk);
            dq_a.push_back(
                Rc::new(
                    RefCell::new(
                        Leaf { 
                            fragment: fragment,
                            parent: None
                        }
                    )
                )
            );
        }

        loop {
            // return a weird empty tree if we weren't given any data
            if dq_a.len() == 0 {
                return Rope {
                    root: Some(
                              Rc::new(
                                  RefCell::new(
                                      Stem {
                                          weight: 0,
                                          parent: None,
                                          left: None,
                                          right: None,
                                      }
                                  )
                              )
                          ),
                    fragment_size: fragment_size,
                }

            // if we're down to one node, pack it up and return it
            } else if dq_a.len() == 1 {
                return Rope {
                    root: Some(dq_a.pop_front().unwrap()),
                    fragment_size: fragment_size,
                };

            } else {
                while !dq_a.is_empty() {
                    let left = dq_a.pop_front().unwrap();
                    let weight = left.borrow().len();

                    let stem_rc = Rc::new(
                        RefCell::new(
                            Stem {
                                weight: weight,
                                parent: None,
                                left: Some(left),
                                right: None,
                            }
                        )
                    );

                    // set the parent of the left child
                    let weak = Rc::downgrade(&stem_rc);

                    match stem_rc.borrow_mut().deref_mut() {
                        &mut Stem { ref left, ref mut right, .. } => {
                            Self::set_parent(left.as_ref().unwrap(), weak.clone());

                            match dq_a.pop_front() {
                                Some(right_child) => {
                                    Self::set_parent(&right_child, weak.clone());
                                    *right = Some(right_child);
                                },
                                _ => (),
                            }
                        },

                        _ => panic!("expected Stem"),
                    }

                    dq_b.push_back(stem_rc);
                }
            }

            dq_swap = dq_a;
            dq_a = dq_b;
            dq_b = dq_swap;
        } // end loop
    }

}

impl <T> Rope<T> {

    /// Get the length of the sequence contained in this Rope.
    pub fn len(&self) -> usize {
        match self.root {
            Some(ref node_rrcc) => node_rrcc.borrow().len(),
            None => 0,
        }
    }

    /// For some index, return the leaf node whose fragment contains that
    /// index, and the offset from zero at the beginning of that node's
    /// fragment.
    fn leaf_and_offset_for_index(&self, mut index: usize) ->
        Option<(Ref<Node<T>>, usize)> {

        // extract the root node, we'll use this as our working pointer
        let mut ptr: Ref<Node<T>> = if self.len() == 0 {
            return None
        } else if let Some(ref ptr) = self.root {
            ptr.as_ref().borrow()
        } else {
            panic!("len 0 but root is None")
        };

        // we'll use this to track a cumulative offset whenever we visit a
        // right child
        let mut offset: usize = 0;

        // loop to traverse down the tree until a Leaf
        loop { // &Stem { weight, ref left, ref right, .. } = ptr.borrow().deref() {

            let weight = match ptr.deref() {
                &Leaf { .. } => break, // we're done descending
                &Stem { weight, .. } => weight,
            };

            // if index is less than the weight of this node, we go down the
            // left subtree. The offset doesn't change.
            if index < weight {

                /*
                let ptr = Ref::map(ptr, |node| {
                    match node {
                        &Stem { ref left 
                })

                let tmp_ptr = match ptr.as_ref().deref() {
                    &Stem { ref left, ..} => match left {
                        &Some(ref child_ptr) => child_ptr,
                        _ => panic!("found no left subtree when weight > 0"),
                    },
                    _ => panic!("should be Stem"),
                };

                ptr = tmp_ptr;
                */

            } else {

            }


        }

        unimplemented!();
    }

    fn set_parent(child: &Rrcc<Node<T>>, parent_weak: Wrc<Node<T>>) {
        match child.borrow_mut().deref_mut() {
           &mut Stem { ref mut parent, .. } =>
               *parent = Some(parent_weak),
           &mut Leaf { ref mut parent, .. } =>
               *parent = Some(parent_weak),
        }
    }

}

impl <'a, T> Cursor<'a, T> {

    pub fn new<'b: 'a>(rope: &'b Rope<T>) -> Self {
        unimplemented!();
    }

    pub fn get(&self) -> Ref<T> {
        let i_fragment = self.at - self.fragment_start;

        Ref::map(self.ptr.borrow(), |node| {
            match node {
                &Stem { .. } => panic!("expected Leaf"),
                &Leaf { ref fragment, .. } => &fragment[i_fragment],
            }
        })
    }

}


#[cfg(test)]
mod tests;
