// tests/golden/10_traits.rs minus `boxed` (`&dyn Shape` is not valid Rust:
// Shape has an associated const, so it is not dyn compatible).
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

fn main() {
    let v = vec![Square { side: 2.0 }, Square { side: 0.5 }];
    println!("{}", total_area(&v).to_bits());
    println!("{} {}", print_it(&v[0]), <Square as Shape>::SIDES);
    println!("{}", v[1].describe());
}
