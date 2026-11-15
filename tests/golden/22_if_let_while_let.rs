pub fn pick(a: Option<i32>, b: Option<i32>) -> i32 {
    if let Some(x) = a {
        x
    } else if let Some(y) = b {
        y * 10
    } else {
        0
    }
}

pub fn drain(mut v: Vec<i32>) -> i32 {
    let mut total = 0;
    while let Some(x) = v.pop() {
        total += x;
    }
    total
}

pub fn chained(a: Option<i32>, limit: i32) -> bool {
    if let Some(x) = a && x > limit {
        return true;
    }
    false
}

pub fn let_else(a: Option<i32>) -> i32 {
    let Some(x) = a else {
        return 0;
    };
    x
}
