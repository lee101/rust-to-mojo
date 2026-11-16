#[path = "../golden/22_if_let_while_let.rs"]
mod g;
use g::*;

fn main() {
    println!("{} {} {}", pick(Some(3), Some(4)), pick(None, Some(4)), pick(None, None));
    println!("{} {}", drain(vec![1, 2, 3, 4]), drain(vec![]));
    println!("{} {} {}", chained(Some(5), 3) as i32, chained(Some(2), 3) as i32, chained(None, 3) as i32);
}
