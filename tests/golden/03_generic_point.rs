use core::fmt::Debug;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point<T: Clone + Debug + Default + PartialEq> {
    pub x: T,
    pub y: T,
}

pub const fn point<T: Clone + Debug + Default + PartialEq>(x: T, y: T) -> Point<T> {
    Point { x, y }
}

impl<T: Clone + Debug + Default + PartialEq> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn swap(&self) -> Self {
        Point { x: self.y.clone(), y: self.x.clone() }
    }
}

impl<T: Clone + Debug + Default + PartialEq + PartialOrd> Point<T> {
    pub fn max_coord(&self) -> T {
        if self.x > self.y {
            self.x.clone()
        } else {
            self.y.clone()
        }
    }
}

impl Point<f32> {
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn scale(&self, factor: f32) -> Point<f32> {
        Point { x: self.x * factor, y: self.y * factor }
    }
}
