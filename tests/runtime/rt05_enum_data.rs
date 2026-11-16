#[path = "../golden/05_enum_data.rs"]
mod g;
use g::*;

fn main() {
    let shapes = vec![Shape::Circle(2.0), Shape::Circle(11.0), Shape::Rect { w: 2.0, h: 3.5 }, unit_square(), nothing()];
    for s in &shapes {
        println!("{} {}", s.area().to_bits(), s.describe());
    }
    println!("{} {}", (unit_square() == Shape::Rect { w: 1.0, h: 1.0 }) as i32, (nothing() == unit_square()) as i32);
}
