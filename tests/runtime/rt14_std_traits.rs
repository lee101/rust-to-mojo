#[path = "../golden/14_std_traits.rs"]
mod g;
use g::*;

fn main() {
    let w = white();
    println!("{} {:?}", w, w);
    let c = Rgb { r: 1, g: 2, b: 3 };
    println!("{} {:?}", c, c);
    println!("{} {}", black(), fallback());
    println!("{}", (black() == fallback()) as i32);
    println!("{}", Rgb::from(0x123456));
}
