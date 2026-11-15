//! A tiny crate exercising crate mode.
mod geometry;
pub mod color;

mod util {
    pub fn clamp01(x: f32) -> f32 {
        if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x }
    }
}

pub use geometry::Point;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
