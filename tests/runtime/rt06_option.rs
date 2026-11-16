#[path = "../golden/06_option.rs"]
mod g;
use g::*;

fn show(o: Option<i32>) -> String {
    match o {
        Some(v) => format!("Some({})", v),
        None => String::from("None"),
    }
}

fn main() {
    println!("{}", show(first_positive(&vec![-3, 0, 7, 9])));
    println!("{}", show(first_positive(&vec![-3, 0])));
    println!("{} {}", double_or_zero(Some(21)), double_or_zero(None));
    println!("{} {} {}", show(add_opts(Some(2), Some(3))), show(add_opts(None, Some(3))), show(add_opts(Some(2), None)));
    println!("{} {}", describe(Some(1.5)).to_bits(), describe(None).to_bits());
    println!("{} {}", has(&Some(String::from("a"))) as i32, has(&None) as i32);
}
