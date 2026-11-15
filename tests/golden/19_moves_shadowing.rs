pub struct Buffer {
    pub data: Vec<u8>,
    pub name: String,
}

pub fn wrap(data: Vec<u8>, name: String) -> Buffer {
    Buffer { data, name }
}

pub fn rename(b: Buffer, name: String) -> Buffer {
    Buffer { data: b.data, name }
}

pub fn shadow(x: i32) -> i32 {
    let x = x + 1;
    let x = x * 2;
    let y = {
        let x = x - 1;
        x
    };
    x + y
}

pub fn takes_twice(v: Vec<u8>) -> usize {
    let n = v.len();
    let w = v;
    n + w.len()
}
