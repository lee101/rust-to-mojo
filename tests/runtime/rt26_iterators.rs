#[path = "../golden/26_iterators.rs"]
mod g;
use g::*;

fn main() {
    let e = evens_squared(&vec![1, 2, 3, 4, -6]);
    println!("{} {} {} {}", e.len(), e[0], e[1], e[2]);
    println!("{}", total(&[0.5, 0.25, 1.0]).to_bits());
    println!("{}", count_big(&vec![1, 5, 10, 20], 5));
    println!("{} {}", all_positive(&vec![1.0, 2.0]) as i32, all_positive(&vec![1.0, -2.0]) as i32);
    print_all(&vec![String::from("p"), String::from("q")]);
    println!("{}", sum_of_range(10));
}
