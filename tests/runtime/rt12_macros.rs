// `log::debug!` resolves to this stand-in (the golden needs no real logger).
mod log {
    macro_rules! debug {
        ($($t:tt)*) => {};
    }
    pub(crate) use debug;
}
include!("../golden/12_macros.rs");

fn main() {
    println!("{}", greet("bob", 3));
    println!("{}", named_args(1.5));
    println!("{}", named_args(2.0));
    println!("{}", named_args(0.1));
    println!("{}", debug_fmt(&vec![1, -2, 3]));
    shout("hey");
    println!("{} {} {}", checked(5), never(0), logs(9));
}
