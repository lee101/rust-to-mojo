pub fn sum_to(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        total += i;
    }
    total
}

pub fn sum_odd_inclusive(a: i64, b: i64) -> i64 {
    let mut s = 0;
    for i in a..=b {
        if i % 2 == 0 {
            continue;
        }
        s += i;
    }
    s
}

pub fn countdown(mut n: i32) -> i32 {
    let mut steps = 0;
    while n > 0 {
        n -= 1;
        steps += 1;
    }
    steps
}

pub fn find_first_over(limit: i32) -> i32 {
    let mut x = 1;
    loop {
        x *= 2;
        if x > limit {
            break;
        }
    }
    x
}

pub fn evens(n: usize) -> usize {
    let mut c = 0;
    for _ in (0..n).step_by(2) {
        c += 1;
    }
    for i in (0..n).rev() {
        c += i;
    }
    c
}

pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

pub fn classify(x: i32) -> &'static str {
    let label = if x > 100 { "big" } else { "small" };
    label
}
