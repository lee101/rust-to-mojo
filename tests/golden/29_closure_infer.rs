//! Closure parameter types inferred from the call site.

pub struct Window {
    pub width: f32,
}

pub struct App {
    pub windows: Vec<Window>,
    pub scale: f32,
}

pub fn apply<F: Fn(f32) -> f32>(x: f32, f: F) -> f32 {
    f(x)
}

pub fn with_window(w: &mut Window, f: impl FnOnce(&mut Window)) {
    f(w)
}

pub fn count_if<F>(xs: &[i32], pred: F) -> usize
where
    F: Fn(&i32) -> bool,
{
    let mut n = 0;
    for x in xs {
        if pred(x) {
            n += 1;
        }
    }
    n
}

impl App {
    pub fn update_scale(&mut self, f: impl Fn(f32) -> f32) {
        self.scale = f(self.scale);
    }

    pub fn demo(&mut self) -> f32 {
        let k: f32 = 2.0;
        let a = apply(3.0, |x| x * k);
        self.update_scale(|s| s + 1.0);
        a
    }
}

pub fn grow(w: &mut Window) {
    with_window(w, |win| win.width += 10.0);
}

pub fn positives(xs: &[i32]) -> usize {
    count_if(xs, |&x| x > 0)
}

pub fn first_len(name: Option<String>) -> usize {
    name.map(|n| n.len()).unwrap_or(0)
}

pub fn or_default(v: Option<i32>) -> i32 {
    v.unwrap_or_else(|| -1)
}

pub fn is_big(v: Option<i32>) -> bool {
    v.is_some_and(|x| x > 100)
}

/// No context to infer from: stays a stub.
pub fn opaque() -> i32 {
    let f = |x| x + 1;
    f(1)
}
