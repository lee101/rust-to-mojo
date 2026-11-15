/// A 2D counter with a name.
#[derive(Clone, Debug, PartialEq)]
pub struct Counter {
    /// Current count.
    pub count: i64,
    pub step: i64,
    pub label: String,
}

impl Counter {
    /// Creates a counter starting at zero.
    pub fn new(label: String, step: i64) -> Self {
        Counter { count: 0, step, label }
    }

    pub fn get(&self) -> i64 {
        self.count
    }

    pub fn tick(&mut self) {
        self.count += self.step;
    }

    pub fn reset(&mut self) -> i64 {
        let old = self.count;
        self.count = 0;
        old
    }

    pub fn is_positive(&self) -> bool {
        self.count > 0 && self.step != 0
    }
}

pub fn make_default() -> Counter {
    Counter::new(String::from("default"), 1)
}
