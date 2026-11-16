#[path = "../golden/15_color.rs"]
mod g;
use g::*;

fn pr(c: Rgba) {
    println!("{} {} {} {}", c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits());
}

fn ph(c: Hsla) {
    println!("{} {} {} {}", c.h.to_bits(), c.s.to_bits(), c.l.to_bits(), c.a.to_bits());
}

fn main() {
    let a = rgba(0x336699ff);
    let b = rgba(0xff000080);
    pr(a);
    pr(a.blend(b));
    pr(a.blend(rgba(0x00ff0000)));
    pr(a.blend(rgba(0x0000ffff)));
    for i in 0..13 {
        let h = Hsla { h: i as f32 / 12.0, s: 0.75, l: 0.4, a: 0.9 };
        pr(Rgba::from(h));
    }
    let h = Hsla { h: 0.3, s: 0.5, l: 0.6, a: 0.8 };
    ph(h.opacity(0.5));
    ph(h.opacity(7.0));
    ph(h.grayscale());
    println!("{} {}", h.is_transparent() as i32, transparent_black().is_transparent() as i32);
    println!("{}", (a == rgba(0x336699ff)) as i32);
}
