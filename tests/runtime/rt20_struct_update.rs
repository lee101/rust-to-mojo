#[path = "../golden/20_struct_update.rs"]
mod g;
use g::*;

fn p(s: Style) {
    println!("{} {} {}", s.width.to_bits(), s.height.to_bits(), s.visible as i32);
}
fn main() {
    p(wide());
    p(hidden(Style { width: 3.0, height: 4.0, visible: true }));
    p(from_fn());
}
