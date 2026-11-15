pub struct Pair {
    pub a: i32,
    pub b: i32,
}

pub fn swap(p: (i32, i32)) -> (i32, i32) {
    let (x, y) = p;
    (y, x)
}

pub fn sum_pair(p: &Pair) -> i32 {
    let Pair { a, b } = p;
    a + b
}

pub fn bucket(n: u32) -> u32 {
    match n {
        0 => 0,
        1..=9 => 1,
        10..=99 => 2,
        _ => 3,
    }
}

pub fn quadrant(p: (f32, f32)) -> u8 {
    match p {
        (x, y) if x >= 0.0 && y >= 0.0 => 1,
        (x, _) if x < 0.0 => 2,
        _ => 3,
    }
}

pub fn nested(o: Option<Option<i32>>) -> i32 {
    match o {
        Some(Some(v)) => v,
        Some(None) => -1,
        None => -2,
    }
}

pub fn first_of(xs: &[i32]) -> i32 {
    match xs {
        [first, ..] => *first,
        [] => 0,
    }
}
