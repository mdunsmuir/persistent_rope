use super::*;

pub fn sample_flat_rope() -> Rope<usize> {
    Rope::new(vec![0, 1, 2])
}

pub fn sample_deep_rope() -> Rope<usize> {
    let v1 = vec![0, 1, 2];
    let v2 = vec![3, 4, 5];
    let v3 = vec![6, 7, 8];

    Rope::concat(Rope::new(v1), Rope::concat(Rope::new(v2), Rope::new(v3)))
}

#[test]
fn length() {
    let empty_rope = Rope::new(Vec::new(): Vec<usize>);
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

mod substring {

    use super::*;

    #[test]
    fn flat() {
        let base = sample_flat_rope();
        let sub = base.substring(1, 3);
        assert_eq!(vec![1, 2], sub.iter().cloned().collect(): Vec<usize>);
    }

    #[test]
    fn deep() {
        let base = sample_deep_rope();
        let sub = base.substring(1, 5);
        assert_eq!(vec![1, 2, 3, 4], sub.iter().cloned().collect(): Vec<usize>);
    }
}
