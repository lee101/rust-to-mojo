use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Node {
    pub value: i32,
    pub visits: Cell<u32>,
}

pub fn shared(v: i32) -> Rc<RefCell<Vec<i32>>> {
    let data = Rc::new(RefCell::new(vec![v]));
    let other = Rc::clone(&data);
    other.borrow_mut().push(v + 1);
    data
}

pub fn count(n: &Node) -> u32 {
    n.visits.set(n.visits.get() + 1);
    n.visits.get()
}
