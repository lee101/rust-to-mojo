#[path = "../golden/18_integers.rs"]
mod g;
use g::*;

fn show(o: Option<u32>) -> String {
    match o {
        Some(v) => format!("Some({})", v),
        None => String::from("None"),
    }
}

fn main() {
    println!("{} {} {} {}", rem_signed(7, 3), rem_signed(-7, 3), rem_signed(7, -3), rem_signed(-7, -3));
    println!("{} {}", rem_float(-7.5, 2.0).to_bits(), rem_float(7.5, -2.0).to_bits());
    println!("{} {}", wrapping(250, 10), wrapping(1, 2));
    println!("{} {}", show(checked(u32::MAX, 1)), show(checked(1, 2)));
    println!("{} {}", saturating(-32000, 1000), saturating(5, 3));
    println!("{} {} {}", casts(3.9), casts(-1.5), casts(300.0));
    println!("{} {}", bits(0x12345678), bits(0));
    println!(
        "{} {} {} {}",
        bools(true, true) as i32,
        bools(true, false) as i32,
        bools(false, true) as i32,
        bools(false, false) as i32
    );
    println!("{}", literals().to_bits());
    println!("{}", maxes());
    println!("{}", pi_ish().to_bits());
}
