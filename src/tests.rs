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

mod indexing {

    use super::*;

    #[test]
    fn flat() {
        let rope = sample_flat_rope();
        for i in 0..2 {
            assert_eq!(i, *rope.at(i));
        }
    }

    #[test]
    fn deep() {
        let rope = sample_deep_rope();
        for i in 0..8 {
            assert_eq!(i, *rope.at(i));
        }
    }

    #[test]
    #[should_panic]
    fn flat_panic_on_out_of_bounds() {
        sample_flat_rope().at(3);
    }

    #[test]
    #[should_panic]
    fn deep_panic_on_out_of_bounds() {
        sample_flat_rope().at(9);
    }
}

mod substring {

    use super::*;

    /*
    #[test]
    fn flat() {
        let base = sample_flat_rope();

        
    }
    */

    #[test]
    fn deep() {
        let base = sample_deep_rope();
        let sub = base.substring(1, 5);
        for i in 0..4 {
            assert_eq!(base.at(i + 1), sub.at(i))
        }
    }
}
