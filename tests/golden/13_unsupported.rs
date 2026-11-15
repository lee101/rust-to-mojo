use std::ffi::c_void;

pub unsafe fn raw_read(p: *const u8) -> u8 {
    *p
}

pub async fn fetch() -> u32 {
    42
}

static mut COUNTER: u32 = 0;

extern "C" {
    fn abs(x: i32) -> i32;
}

pub fn dynamic(items: Vec<Box<dyn Fn(i32) -> i32>>) -> i32 {
    items.len() as i32
}

pub trait Describe {
    fn describe(&self) -> String;
}

impl Describe for i32 {
    fn describe(&self) -> String {
        format!("int {}", self)
    }
}

impl<T: Clone> Describe for Vec<T> {
    fn describe(&self) -> String {
        "vec".to_string()
    }
}

pub struct Holder<'a> {
    pub data: &'a [u8],
}

pub union IntOrFloat {
    i: u32,
    f: f32,
}

pub fn label_loop(n: i32) -> i32 {
    let mut c = 0;
    'outer: for i in 0..n {
        for j in 0..n {
            if i * j > 10 {
                break 'outer;
            }
            c += 1;
        }
    }
    c
}
