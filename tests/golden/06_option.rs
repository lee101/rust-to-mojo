pub fn first_positive(values: &Vec<i32>) -> Option<i32> {
    for v in values {
        if *v > 0 {
            return Some(*v);
        }
    }
    None
}

pub fn double_or_zero(x: Option<i32>) -> i32 {
    match x {
        Some(v) => v * 2,
        None => 0,
    }
}

pub fn add_opts(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    let x = a?;
    let y = b?;
    Some(x + y)
}

pub fn describe(x: Option<f32>) -> f32 {
    if let Some(v) = x {
        v + 1.0
    } else {
        x.unwrap_or(-1.0)
    }
}

pub fn has(x: &Option<String>) -> bool {
    x.is_some()
}
