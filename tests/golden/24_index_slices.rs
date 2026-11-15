use std::ops::Index;

pub struct Grid {
    pub cells: Vec<f32>,
    pub width: usize,
}

impl Index<usize> for Grid {
    type Output = f32;
    fn index(&self, i: usize) -> &f32 {
        &self.cells[i]
    }
}

impl Grid {
    pub fn row(&self, r: usize) -> &[f32] {
        &self.cells[r * self.width..(r + 1) * self.width]
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.cells[y * self.width + x]
    }

    pub fn first(&self) -> &f32 {
        &self.cells[0]
    }
}
