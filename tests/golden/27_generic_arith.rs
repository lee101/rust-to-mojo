//! Generic arithmetic: operators on a type parameter go through mojo_rt.ops.
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixels(pub f32);

impl Add for Pixels {
    type Output = Pixels;
    fn add(self, rhs: Pixels) -> Pixels {
        Pixels(self.0 + rhs.0)
    }
}

impl Sub for Pixels {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Pixels(self.0 - rhs.0)
    }
}

impl Mul<f32> for Pixels {
    type Output = Pixels;
    fn mul(self, rhs: f32) -> Pixels {
        Pixels(self.0 * rhs)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point<T: Clone + Default> {
    pub x: T,
    pub y: T,
}

impl<T: Clone + Default + Add<Output = T>> Add for Point<T> {
    type Output = Point<T>;
    fn add(self, rhs: Self) -> Self {
        Point { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl<T: Clone + Default + Neg<Output = T>> Neg for Point<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Point { x: -self.x, y: -self.y }
    }
}

impl<T: Clone + Default + AddAssign> AddAssign for Point<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T> Point<T>
where
    T: Clone + Default + Add<T, Output = T> + Sub<Output = T>,
{
    pub fn sum(&self) -> T {
        self.x.clone() + self.y.clone()
    }

    pub fn delta(&self, other: &Point<T>) -> Point<T> {
        Point { x: other.x.clone() - self.x.clone(), y: other.y.clone() - self.y.clone() }
    }
}

impl<T: Clone + Default + Mul<f32, Output = T>> Point<T> {
    /// Scaling by `f32`: `ops.mul_rhs` (same type, or `T: RMulF32`).
    pub fn scale(&self, factor: f32) -> Point<T> {
        Point { x: self.x.clone() * factor, y: self.y.clone() * factor }
    }
}

pub fn double<T: Add<Output = T> + Copy>(v: T) -> T {
    v + v
}

/// Heterogeneous `Add<u8>`: no dispatch trait, stays a TODO.
pub fn bump<T: Add<u8, Output = T>>(v: T) -> T {
    v + 1u8
}
