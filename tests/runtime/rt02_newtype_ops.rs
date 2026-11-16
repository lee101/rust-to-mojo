#[path = "../golden/02_newtype_ops.rs"]
mod g;
use g::*;

fn b(x: Pixels) -> u32 {
    x.0.to_bits()
}

fn main() {
    let a = px(1.5);
    let c = px(-2.25);
    println!("{} {} {}", b(a + c), b(a - c), b(-a));
    println!("{} {}", b(a * 3.0), b(2.0 * c));
    let mut d = a;
    d += c;
    println!("{}", b(d));
    for v in [2.5f32, -2.5, 0.5, -0.5, 1.4999, 3.7, -3.7] {
        let p = px(v);
        println!("{} {} {}", b(p.floor()), b(p.round()), b(p.abs()));
    }
    println!("{} {}", b(a.max(c)), b(px(f32::NAN).max(a)));
    println!("{} {} {}", (a < c) as i32, (c < a) as i32, (a == px(1.5)) as i32);
    println!("{}", b(Pixels::ZERO));
}
