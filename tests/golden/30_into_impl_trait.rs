//! `.into()` targets from argument/variant types, `-> impl Trait` with a
//! concrete tail, and impls of local traits for primitives.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixels(pub f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rems(pub f32);

impl From<f32> for Pixels {
    fn from(v: f32) -> Self {
        Pixels(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Px(Pixels),
    Rem(Rems),
}

pub fn px_len(v: f32) -> Length {
    Length::Px(v.into())
}

pub fn width_of(p: Pixels) -> f32 {
    p.0
}

pub fn measure(v: f32) -> f32 {
    width_of(v.into())
}

pub trait Shape {
    fn area(&self) -> f32;
}

pub struct Square {
    pub side: f32,
}

impl Shape for Square {
    fn area(&self) -> f32 {
        self.side * self.side
    }
}

pub fn unit_square() -> impl Shape {
    Square { side: 1.0 }
}

pub trait Half {
    fn half(&self) -> Self;
}

impl Half for f32 {
    fn half(&self) -> Self {
        self / 2.0
    }
}

/// No concrete tail type: stays a stub.
pub fn evens(n: u32) -> impl Iterator<Item = u32> {
    (0..n).filter(|x| x % 2 == 0)
}

/// `Option<&T>` arguments become `Optional[T]` (a copy of the referent).
pub fn label_len(label: Option<&String>) -> usize {
    match label {
        Some(l) => l.len(),
        None => 0,
    }
}
