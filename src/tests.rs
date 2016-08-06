use super::*;

pub fn sample_flat_rope() -> Rope<usize> {
    Rope::new(&vec![0, 1, 2])
}

pub fn sample_deep_rope() -> Rope<usize> {
    let v1 = &vec![0, 1, 2];
    let v2 = &vec![3, 4, 5];
    let v3 = &vec![6, 7, 8];

    Rope::concat(&Rope::new(v1), &Rope::concat(&Rope::new(v2), &Rope::new(v3)))
}

#[test]
fn length() {
    let empty_rope: Rope<usize> = Rope::new(&(Vec::new(): Vec<usize>));
    assert_eq!(0, empty_rope.len());
    assert!(empty_rope.is_empty());

    let flat_rope = sample_flat_rope();
    assert_eq!(3, flat_rope.len());
    assert!(!flat_rope.is_empty());

    let deep_rope = sample_deep_rope();
    assert_eq!(9, deep_rope.len());
    assert!(!deep_rope.is_empty());
}

mod indexing {

    use super::*;

    #[test]
    fn flat() {
        let rope = sample_flat_rope();
        for i in 0..2 {
            assert_eq!(i, rope[i]);
        }
    }

    #[test]
    fn deep() {
        let rope = sample_deep_rope();
        for i in 0..8 {
            assert_eq!(i, rope[i]);
        }
    }

    #[test]
    #[should_panic]
    fn flat_panic_on_out_of_bounds() {
        sample_flat_rope()[3];
    }

    #[test]
    #[should_panic]
    fn deep_panic_on_out_of_bounds() {
        sample_flat_rope()[9];
    }
}

mod iteration {

    use super::*;

    #[test]
    fn flat() {
        let rope = sample_flat_rope();
        assert_eq!(vec![0, 1, 2], rope.iter().cloned().collect(): Vec<usize>);
    }

    #[test]
    fn deep() {
        let rope = sample_deep_rope();
        let exp = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(exp, rope.iter().cloned().collect(): Vec<usize>);
    }

}

mod slice {

    use super::*;

    #[test]
    fn flat() {
        let base = sample_flat_rope();
        let sub = base.slice(1, 3);
        assert_eq!(vec![1, 2], sub.iter().cloned().collect(): Vec<usize>);
    }

    #[test]
    fn deep() {
        let base = sample_deep_rope();
        let sub = base.slice(1, 5);
        assert_eq!(vec![1, 2, 3, 4], sub.iter().cloned().collect(): Vec<usize>);
    }
}

mod markers {

    use super::super::*;

    #[derive(PartialEq, Eq, Hash, Clone, Copy)]
    struct Marker {}

    fn flat_marked_rope() -> Rope<usize, Marker> {
        let mut chunk = Chunk::with_capacity(3);
        chunk.extend_from_slice(&vec![0, 1, 2]);
        chunk.mark_at(Marker {}, 1);
        Rope::from_chunk(chunk)
    }

    fn deep_marked_rope() -> Rope<usize, Marker> {
        let mut chunk = Chunk::with_capacity(3);
        chunk.extend_from_slice(&vec![0, 1, 2, 3]);
        chunk.mark_at(Marker {}, 1);
        chunk.mark_at(Marker {}, 3);
        let rope = Rope::from_chunk(chunk);

        Rope::concat(&flat_marked_rope(),
                     &Rope::concat(&rope, &flat_marked_rope()))
    }

    #[test]
    fn flat_indices() {
        assert_eq!(Some(1), flat_marked_rope().index_for_nth_marker(Marker {}, 0));
        assert_eq!(None, flat_marked_rope().index_for_nth_marker(Marker {}, 1));
        assert_eq!(None, flat_marked_rope().index_for_nth_marker(Marker {}, 25));
    }

    #[test]
    fn deep_indices() {
        assert_eq!(Some(1), deep_marked_rope().index_for_nth_marker(Marker {}, 0));
        assert_eq!(Some(4), deep_marked_rope().index_for_nth_marker(Marker {}, 1));
        assert_eq!(Some(6), deep_marked_rope().index_for_nth_marker(Marker {}, 2));
        assert_eq!(Some(8), deep_marked_rope().index_for_nth_marker(Marker {}, 3));
        assert_eq!(None, deep_marked_rope().index_for_nth_marker(Marker {}, 4));
        assert_eq!(None, deep_marked_rope().index_for_nth_marker(Marker {}, 25));
    }

    #[test]
    fn slice_count() {
        assert_eq!(None,
                   flat_marked_rope().slice(0, 1).marker_counts().get(&Marker{}));

        assert_eq!(Some(&1),
                   flat_marked_rope().slice(1, 3).marker_counts().get(&Marker{}));

        assert_eq!(None,
                   deep_marked_rope().slice(0, 1).marker_counts().get(&Marker{}));

        assert_eq!(Some(&1),
                   deep_marked_rope().slice(1, 3).marker_counts().get(&Marker{}));

        assert_eq!(Some(&2),
                   deep_marked_rope().slice(1, 6).marker_counts().get(&Marker{}));
    }

    #[test]
    fn flat_count() {
        assert_eq!(Some(&1), flat_marked_rope().marker_counts().get(&Marker{}));
    }

    #[test]
    fn deep_count() {
        assert_eq!(Some(&4), deep_marked_rope().marker_counts().get(&Marker{}));
    }
}
