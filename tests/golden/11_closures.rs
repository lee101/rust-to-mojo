pub fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

pub fn apply_twice(f: impl Fn(f32) -> f32, x: f32) -> f32 {
    f(f(x))
}

pub fn typed_closure(k: i32) -> i32 {
    let add = |x: i32| -> i32 { x + 1 };
    add(k)
}

pub fn capturing(k: i32) -> i32 {
    let scale = |x: i32| -> i32 { x * k };
    scale(3)
}

pub fn untyped(v: &Vec<i32>) -> Vec<i32> {
    v.iter().map(|x| x * 2).collect()
}

pub fn any_negative(v: &Vec<i32>) -> bool {
    v.iter().any(|x| *x < 0)
}
