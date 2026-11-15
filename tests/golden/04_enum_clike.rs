/// Axis in a 2D cartesian space.
#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd)]
pub enum Axis {
    /// The y axis
    Vertical,
    /// The x axis
    Horizontal,
}

impl Axis {
    pub fn invert(self) -> Self {
        match self {
            Axis::Vertical => Axis::Horizontal,
            Axis::Horizontal => Axis::Vertical,
        }
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, Axis::Vertical)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Corner {
    #[default]
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 4,
    BottomRight,
}

pub fn corner_code(c: Corner) -> i32 {
    match c {
        Corner::TopLeft | Corner::TopRight => 1,
        Corner::BottomLeft => 2,
        _ => 3,
    }
}
