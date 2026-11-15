pub fn greet(name: &str, n: i32) -> String {
    format!("hello {}, you are #{}", name, n)
}

pub fn named_args(x: f32) -> String {
    format!("x={x} doubled={d}", d = x * 2.0)
}

pub fn debug_fmt(v: &Vec<i32>) -> String {
    format!("{:?}", v)
}

pub fn precise(x: f32) -> String {
    format!("{:.2}", x)
}

pub fn shout(msg: &str) {
    println!("{}!", msg);
    print!("no newline");
    println!();
}

pub fn checked(x: i32) -> i32 {
    assert!(x >= 0, "x must be non-negative, got {}", x);
    assert_eq!(x % 1, 0);
    debug_assert!(x < 1000);
    if x > 100 {
        panic!("too big: {}", x);
    }
    x
}

pub fn never(x: i32) -> i32 {
    match x {
        0 => 1,
        _ => unreachable!(),
    }
}

pub fn logs(x: i32) -> i32 {
    log::debug!("x = {}", x);
    x + 1
}

macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

pub fn uses_macro(x: i32) -> i32 {
    square!(x)
}
