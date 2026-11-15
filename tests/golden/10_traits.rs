pub trait Shape {
    type Unit;
    const SIDES: u32;

    fn area(&self) -> f64;

    fn describe(&self) -> String {
        format!("shape with area {}", self.area())
    }
}

pub trait Named: Shape {
    fn name(&self) -> String;
}

#[derive(Clone, Copy)]
pub struct Square {
    pub side: f64,
}

impl Shape for Square {
    type Unit = f64;
    const SIDES: u32 = 4;

    fn area(&self) -> f64 {
        self.side * self.side
    }
}

impl Named for Square {
    fn name(&self) -> String {
        "square".to_string()
    }
}

pub fn total_area<S: Shape>(shapes: &Vec<S>) -> f64 {
    let mut t = 0.0;
    for s in shapes {
        t += s.area();
    }
    t
}

pub fn print_it(s: &impl Named) -> String {
    s.name()
}

pub fn boxed(s: &dyn Shape<Unit = f64>) -> f64 {
    s.area()
}
