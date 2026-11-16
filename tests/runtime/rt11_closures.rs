#[path = "../golden/11_closures.rs"]
mod g;
use g::*;

fn dbl(x: i32) -> i32 { x * 2 }
fn halve(x: f32) -> f32 { x / 2.0 }
fn main() {
    println!("{} {}", apply(dbl, 21), apply_twice(halve, 10.0).to_bits());
    println!("{} {}", typed_closure(41), capturing(7));
    let u = untyped(&vec![1, -2, 3]);
    println!("{} {} {} {}", u.len(), u[0], u[1], u[2]);
    println!("{} {}", any_negative(&vec![1, 2]) as i32, any_negative(&vec![1, -2]) as i32);
}
