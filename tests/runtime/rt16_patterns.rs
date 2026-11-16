#[path = "../golden/16_patterns.rs"]
mod g;
use g::*;

fn main() {
    let (x, y) = swap((1, 2));
    println!("{} {}", x, y);
    println!("{}", sum_pair(&Pair { a: 40, b: 2 }));
    for n in [0u32, 1, 9, 10, 99, 100] {
        print!("{} ", bucket(n));
    }
    println!();
    println!("{} {} {}", quadrant((1.0, 1.0)), quadrant((-1.0, 5.0)), quadrant((1.0, -1.0)));
    println!("{} {} {}", nested(Some(Some(5))), nested(Some(None)), nested(None));
}
