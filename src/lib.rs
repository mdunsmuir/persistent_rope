pub mod persistent_rope {

    use std::rc::*;
    use std::cell::RefCell;
    use std::ops::DerefMut;
    use std::collections::VecDeque;

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

    use self::Node::*;

    impl <T> Node<T> {

        /// The length of the string stored under this Node
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

    }

    #[derive(Debug)]
    pub struct Rope<T> {
        root: DownLink<T>,
        fragment_size: usize,
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

        fn set_parent(child: &Rrcc<Node<T>>, parent_weak: Wrc<Node<T>>) {
            match child.borrow_mut().deref_mut() {
               &mut Stem { ref mut parent, .. } =>
                   *parent = Some(parent_weak),
               &mut Leaf { ref mut parent, .. } =>
                   *parent = Some(parent_weak),
            }
        }

    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
