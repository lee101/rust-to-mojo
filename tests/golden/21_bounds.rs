use std::fmt::Debug;
use std::ops::{Add, Sub};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Point<T: Clone + Debug + Default + PartialEq> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Size<T: Clone + Debug + Default + PartialEq> {
    pub width: T,
    pub height: T,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Bounds<T: Clone + Debug + Default + PartialEq> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

impl<T> Bounds<T>
where
    T: Clone + Debug + Default + PartialEq + Add<T, Output = T> + Sub<Output = T>,
{
    pub fn right(&self) -> T {
        self.origin.x.clone() + self.size.width.clone()
    }

    pub fn bottom(&self) -> T {
        self.origin.y.clone() + self.size.height.clone()
    }
}

impl<T> Bounds<T>
where
    T: Clone + Debug + Default + PartialEq + PartialOrd + Add<T, Output = T>,
{
    pub fn contains(&self, point: &Point<T>) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x.clone() + self.size.width.clone()
            && point.y >= self.origin.y
            && point.y <= self.origin.y.clone() + self.size.height.clone()
    }
}

impl<T: Clone + Debug + Default + PartialEq + Add<Output = T>> Add for Point<T> {
    type Output = Point<T>;
    fn add(self, rhs: Self) -> Self {
        Point { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}
