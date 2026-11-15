use std::collections::HashMap;

pub fn squares(n: usize) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for i in 0..n {
        out.push(i * i);
    }
    out
}

pub fn total(values: &[f32]) -> f32 {
    let mut t = 0.0;
    for v in values.iter() {
        t += v;
    }
    t
}

pub fn indexed(values: &Vec<i32>) -> i32 {
    let mut acc = 0;
    for (i, v) in values.iter().enumerate() {
        acc += (i as i32) * v;
    }
    if values.is_empty() {
        return -1;
    }
    acc + values[0] + values.len() as i32
}

pub fn word_counts(words: &Vec<String>) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for w in words {
        if counts.contains_key(w) {
            let c = counts.get(w).unwrap();
            counts.insert(w.clone(), c + 1);
        } else {
            counts.insert(w.clone(), 1);
        }
    }
    counts
}

pub fn sum_map(m: &HashMap<String, i32>) -> i32 {
    let mut s = 0;
    for (_k, v) in m {
        s += v;
    }
    s
}

pub fn literal() -> Vec<i32> {
    let v = vec![1, 2, 3];
    v
}
