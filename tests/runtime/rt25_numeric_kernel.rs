#[path = "../golden/25_numeric_kernel.rs"]
mod g;
use g::*;

fn main() {
    let n = 4;
    let a: Vec<f32> = (0..n * n).map(|i| (i as f32) * 0.5 - 3.0).collect();
    let b: Vec<f32> = (0..n * n).map(|i| 1.0 / (i as f32 + 1.0)).collect();
    println!("{}", dot(&a, &b).to_bits());
    let mut out = vec![0.0f32; n * n];
    matmul(&a, &b, &mut out, n);
    for v in &out {
        print!("{} ", v.to_bits());
    }
    println!();
    println!("{} {}", lerp(1.0, 3.0, 0.25).to_bits(), lerp(-1.0, 1.0, 0.75).to_bits());
}
