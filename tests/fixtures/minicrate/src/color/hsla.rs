use crate::geometry::Point;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

pub fn at(p: Point) -> Hsla {
    Hsla { h: p.x, s: p.y, l: 0.5, a: 1.0 }
}
