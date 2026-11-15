pub fn rem_signed(a: i32, b: i32) -> i32 {
    a % b
}

pub fn rem_float(a: f64, b: f64) -> f64 {
    a % b
}

pub fn wrapping(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

pub fn checked(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

pub fn saturating(a: i16, b: i16) -> i16 {
    a.saturating_sub(b)
}

pub fn casts(x: f32) -> i32 {
    let y = x as i32;
    let z = y as u8;
    (z as i32) + (y as i64) as i32
}

pub fn bits(x: u32) -> u32 {
    (!x & 0xFF) << 2 | (x >> 3) ^ 1
}

pub fn bools(a: bool, b: bool) -> bool {
    !a || (a && !b)
}

pub fn literals() -> f64 {
    let a = 1_000u64;
    let b = 2.5e3_f64;
    let c = 0x10;
    (a as f64) + b + (c as f64)
}

pub fn maxes() -> i32 {
    i32::MAX - i32::MIN.abs().max(1)
}

pub fn pi_ish() -> f32 {
    std::f32::consts::PI * 2.0
}
