pub fn evens_squared(v: &Vec<i64>) -> Vec<i64> {
    v.iter().filter(|x| *x % 2 == 0).map(|x| x * x).collect()
}

pub fn total(v: &[f32]) -> f32 {
    v.iter().sum()
}

pub fn count_big(v: &Vec<u32>, limit: u32) -> usize {
    v.iter().filter(|&x| *x > limit).count()
}

pub fn all_positive(v: &Vec<f64>) -> bool {
    v.iter().all(|x| *x > 0.0)
}

pub fn print_all(v: &Vec<String>) {
    v.iter().for_each(|s| println!("{}", s));
}

pub fn sum_of_range(n: u64) -> u64 {
    (0..n).filter(|i| i % 3 == 0).sum::<u64>()
}

pub fn unknown_source(xs: impl Iterator<Item = u8>) -> usize {
    xs.map(|x| x as usize).count()
}

pub fn into_set(v: &Vec<i32>) -> std::collections::HashSet<i32> {
    v.iter().copied().collect()
}
