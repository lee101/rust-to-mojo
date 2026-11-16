#[path = "../golden/29_closure_infer.rs"]
mod g;
use g::*;

fn main() {
    let mut app = App { windows: vec![], scale: 1.5 };
    let a = app.demo();
    println!("{} {}", a.to_bits(), app.scale.to_bits());
    let mut w = Window { width: 5.0 };
    grow(&mut w);
    println!("{}", w.width.to_bits());
    println!("{}", positives(&[1, -2, 3, 0]));
    println!("{} {}", first_len(Some(String::from("héllo"))), first_len(None));
    println!("{} {}", or_default(Some(4)), or_default(None));
    println!("{} {} {}", is_big(Some(101)) as i32, is_big(Some(1)) as i32, is_big(None) as i32);
}
