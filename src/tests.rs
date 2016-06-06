use super::*;

#[test]
fn from_slice() {
    let vec = vec![0; 20];
    Rope::from_slice(vec.as_slice(), 3);
}

#[test]
fn len() {
    let vec = vec![0; 20];
    assert_eq!(20, Rope::from_slice(vec.as_slice(), 3).len());
}
