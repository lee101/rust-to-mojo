#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Circle(f32),
    Rect { w: f32, h: f32 },
    Empty,
}

impl Shape {
    pub fn area(&self) -> f32 {
        match self {
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rect { w, h } => w * h,
            Shape::Empty => 0.0,
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Shape::Circle(r) if *r > 10.0 => "big circle",
            Shape::Circle(_) => "circle",
            Shape::Rect { w, h } if w == h => "square",
            _ => "other",
        }
    }
}

pub fn unit_square() -> Shape {
    Shape::Rect { w: 1.0, h: 1.0 }
}

pub fn nothing() -> Shape {
    Shape::Empty
}
