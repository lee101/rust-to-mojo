#[path = "../golden/04_enum_clike.rs"]
mod g;
use g::*;

fn main() {
    let v = Axis::Vertical;
    let h = v.invert();
    println!("{} {} {}", v.is_vertical() as i32, h.is_vertical() as i32, (h.invert() == v) as i32);
    println!("{} {}", (v < h) as i32, v as i32 + h as i32);
    for c in [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight, Corner::default()] {
        println!("{} {}", corner_code(c), c as i32);
    }
}
