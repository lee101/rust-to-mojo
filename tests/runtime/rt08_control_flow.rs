#[path = "../golden/08_control_flow.rs"]
mod g;
use g::*;

fn main() {
    println!("{} {}", sum_to(10), sum_to(0));
    println!("{} {}", sum_odd_inclusive(-5, 9), sum_odd_inclusive(3, 3));
    println!("{} {}", countdown(7), countdown(-2));
    println!("{} {}", find_first_over(100), find_first_over(0));
    println!("{} {}", evens(7), evens(0));
    println!("{} {} {}", sign(2.5).to_bits(), sign(-0.1).to_bits(), sign(0.0).to_bits());
    println!("{} {}", classify(101), classify(5));
}
