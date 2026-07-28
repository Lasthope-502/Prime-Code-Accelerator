#![allow(clippy::all)]
use napi_derive::napi;
use rayon::prelude::*;
use std::collections::HashMap;

#[napi]
pub fn sum_of_squares(n: i64) -> i64 {
    (0..n).into_par_iter().map(|i| i * i).sum()
}

#[napi]
pub fn fast_range_sum(n: i64) -> i64 {
    (0..n).sum()
}

#[napi]
pub fn fibonacci(n: u32) -> i64 {
    let (mut a, mut b) = (0i64, 1i64);
    for _ in 0..n {
        let t = a;
        a = b;
        b = t + b;
    }
    a
}

#[napi]
pub fn matrix_multiply(a: Vec<f64>, b: Vec<f64>, n: u32) -> Vec<f64> {
    let n = n as usize;
    let mut result = vec![0.0; n * n];
    result.par_chunks_mut(n).enumerate().for_each(|(i, row)| {
        for k in 0..n {
            let aik = a[i * n + k];
            if aik == 0.0 { continue; }
            for j in 0..n {
                row[j] += aik * b[k * n + j];
            }
        }
    });
    result
}

#[napi]
pub fn fast_string_join(parts: Vec<String>) -> String {
    parts.concat()
}

#[napi]
pub fn fast_word_count(words: Vec<String>) -> HashMap<String, i64> {
    let mut counts = HashMap::new();
    for w in words {
        *counts.entry(w).or_insert(0) += 1;
    }
    counts
}

// ===== Step 8: "Cart Effect" ops =====

#[napi]
pub fn batch_sum_of_squares(numbers: Vec<i64>) -> i64 {
    numbers.par_chunks(50_000)
        .map(|c| c.iter().map(|&x| x * x).sum::<i64>())
        .sum()
}

#[napi]
pub fn get_worker_info() -> u32 {
    num_cpus::get() as u32
}