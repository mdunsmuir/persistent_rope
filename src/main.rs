mod lib;
use lib::persistent_rope::*;

fn main() {
    let vec = vec![1, 2, 3, 4, 5];
    let rope = Rope::from_slice(&vec[0..5], 2);
    println!("{:?}", rope);
}
