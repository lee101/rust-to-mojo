use std::ops::{Add, AddAssign, Mul, Neg, Sub};

/// Logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Pixels(pub f32);

impl Add for Pixels {
    type Output = Pixels;
    fn add(self, rhs: Pixels) -> Pixels {
        Pixels(self.0 + rhs.0)
    }
}

impl Sub for Pixels {
    type Output = Pixels;
    fn sub(self, rhs: Self) -> Self::Output {
        Pixels(self.0 - rhs.0)
    }
}

impl Mul<f32> for Pixels {
    type Output = Pixels;
    fn mul(self, rhs: f32) -> Pixels {
        Pixels(self.0 * rhs)
    }
}

impl Mul<Pixels> for f32 {
    type Output = Pixels;
    fn mul(self, rhs: Pixels) -> Pixels {
        Pixels(self * rhs.0)
    }
}

impl Neg for Pixels {
    type Output = Pixels;
    fn neg(self) -> Pixels {
        Pixels(-self.0)
    }
}

impl AddAssign for Pixels {
    fn add_assign(&mut self, rhs: Pixels) {
        self.0 += rhs.0;
    }
}

impl Pixels {
    pub const ZERO: Pixels = Pixels(0.0);

    pub fn floor(&self) -> Self {
        Self(self.0.floor())
    }

    pub fn round(&self) -> Self {
        Self(self.0.round())
    }

    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub fn max(&self, other: Pixels) -> Pixels {
        Pixels(self.0.max(other.0))
    }
}

pub fn px(value: f32) -> Pixels {
    Pixels(value)
}
