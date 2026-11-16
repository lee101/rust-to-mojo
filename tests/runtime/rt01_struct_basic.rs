#[path = "../golden/01_struct_basic.rs"]
mod g;
use g::*;

fn main() {
    let mut c = Counter::new(String::from("x"), 3);
    c.tick();
    c.tick();
    println!("{} {}", c.get(), c.is_positive() as i32);
    let old = c.reset();
    println!("{} {} {}", old, c.get(), c.is_positive() as i32);
    let d = make_default();
    println!("{} {} {}", d.label, d.step, d.count);
}
