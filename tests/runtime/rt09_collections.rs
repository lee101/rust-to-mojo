#[path = "../golden/09_collections.rs"]
mod g;
use g::*;

use std::collections::HashMap;
fn main() {
    let s = squares(5);
    println!("{} {} {}", s.len(), s[2], s[4]);
    println!("{}", total(&[1.5, 2.25, -0.5]).to_bits());
    println!("{} {}", indexed(&vec![3, 4, 5]), indexed(&vec![]));
    let words: Vec<String> = ["a", "b", "a", "c", "a"].iter().map(|s| s.to_string()).collect();
    let c = word_counts(&words);
    println!("{} {} {} {}", c.len(), c["a"], c["b"], c["c"]);
    let mut m: HashMap<String, i32> = HashMap::new();
    m.insert("x".to_string(), 4);
    m.insert("y".to_string(), -9);
    println!("{}", sum_map(&m));
    let l = literal();
    println!("{} {} {}", l[0], l[1], l[2]);
}
