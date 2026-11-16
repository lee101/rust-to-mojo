// included at the crate root so `Tiny`'s private field is visible
include!("../golden/23_drop_clone.rs");

fn main() {
    let h = Handle { id: 1, tag: String::from("t") };
    let c = h.clone();
    println!("clone {} {}", c.id, c.tag);
    drop(c);
    println!("after drop");
    let t = Tiny(7);
    let u = t.clone();
    println!("tiny {}", u.0);
    drop(h);
    println!("end");
}
