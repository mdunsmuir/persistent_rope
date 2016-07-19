use super::*;

fn sample_rope() -> Rope<usize> {
    let v1 = vec![1, 2, 3];
    let v2 = vec![4, 5, 6];
    let v3 = vec![7, 8, 9];

    Rope::concat(Rope::new(v1), Rope::concat(Rope::new(v2), Rope::new(v3)))
}

#[test]
fn basic_indexing() {
    let rope = sample_rope();
    for i in 1..9 {
        assert_eq!(i, *rope.at(i - 1));
    }
}

#[test]
fn substring() {
    let base = sample_rope();
    let sub = base.substring(1, 5);
    for i in 0..4 {
        assert_eq!(base.at(i + 1), sub.at(i))
    }
}
