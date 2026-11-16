#[path = "../golden/21_bounds.rs"]
mod g;
use g::*;

fn main() {
    let b = Bounds { origin: Point { x: 1.5f32, y: -2.0 }, size: Size { width: 3.0, height: 4.5 } };
    println!("{} {}", b.right().to_bits(), b.bottom().to_bits());
    println!("{} {}", b.contains(&Point { x: 2.0, y: 0.0 }) as i32, b.contains(&Point { x: 5.0, y: 0.0 }) as i32);
    let bi = Bounds { origin: Point { x: 10i32, y: 20 }, size: Size { width: 5, height: 7 } };
    println!("{} {} {}", bi.right(), bi.bottom(), bi.contains(&Point { x: 15, y: 27 }) as i32);
    let s = Point { x: 1i64, y: 2 } + Point { x: 10, y: 20 };
    println!("{} {}", s.x, s.y);
    println!("{}", (Point { x: 1i64, y: 2 } == Point::default()) as i32);
}
