#[path = "../golden/27_generic_arith.rs"]
mod g;
use g::*;

fn main() {
    let p = Point { x: Pixels(1.5), y: Pixels(-4.0) };
    let q = Point { x: Pixels(0.25), y: Pixels(2.0) };
    let r = p + q;
    println!("{} {}", r.x.0.to_bits(), r.y.0.to_bits());
    let f = Point { x: 3.5f32, y: -1.25f32 };
    let mut m = f;
    m += Point { x: 0.5f32, y: 2.0f32 };
    println!("{} {}", m.x.to_bits(), m.y.to_bits());
    let nf = -f;
    println!("{} {}", nf.x.to_bits(), nf.y.to_bits());
    println!("{} {}", f.sum().to_bits(), p.sum().0.to_bits());
    let d = p.delta(&q);
    println!("{} {}", d.x.0.to_bits(), d.y.0.to_bits());
    let s = p.scale(2.0);
    let sf = f.scale(0.5);
    println!("{} {} {} {}", s.x.0.to_bits(), s.y.0.to_bits(), sf.x.to_bits(), sf.y.to_bits());
    let i = Point { x: 7i32, y: -3 };
    println!("{} {}", i.sum(), double(21i64));
    println!("{}", double(1.25f64).to_bits());
}
