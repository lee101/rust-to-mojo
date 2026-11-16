#[path = "../golden/30_into_impl_trait.rs"]
mod g;
use g::*;

fn main() {
    let l = px_len(2.5);
    match l {
        Length::Px(p) => println!("px {}", p.0.to_bits()),
        Length::Rem(_) => println!("rem"),
    }
    println!("{}", measure(4.25).to_bits());
    println!("{}", unit_square().area().to_bits());
    println!("{}", 3.0f32.half().to_bits());
    let s = String::from("abcd");
    println!("{} {}", label_len(Some(&s)), label_len(None));
}
