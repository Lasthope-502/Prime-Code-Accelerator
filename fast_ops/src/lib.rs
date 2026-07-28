use pyo3::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

// ===== Basic numeric ops =====

#[pyfunction]
fn sum_of_squares(n: u64) -> u64 {
    (0..n).into_par_iter().map(|i| i.wrapping_mul(i)).sum()
}

#[pyfunction]
fn fast_range_sum(n: u64) -> u64 {
    (0..n).sum()
}

#[pyfunction]
fn heavy_loop(n: u64) -> u64 {
    let mut total: u64 = 0;
    for i in 0..n {
        total = total.wrapping_add(i.wrapping_mul(i));
    }
    total
}

#[pyfunction]
fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    if n % 2 == 0 { return n == 2; }
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 { return false; }
        i += 2;
    }
    true
}

#[pyfunction]
fn count_primes(n: u64) -> u64 {
    (2..n).into_par_iter().filter(|&x| is_prime(x)).count() as u64
}

#[pyfunction]
fn fibonacci(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let t = a;
        a = b;
        b = t.wrapping_add(b);
    }
    a
}

// ===== Matrix / string / collection ops =====

#[pyfunction]
fn matrix_multiply(a: Vec<f64>, b: Vec<f64>, n: usize) -> Vec<f64> {
    let mut result = vec![0.0; n * n];
    result.par_chunks_mut(n).enumerate().for_each(|(i, row)| {
        for k in 0..n {
            let a_ik = a[i * n + k];
            if a_ik == 0.0 { continue; }
            for j in 0..n {
                row[j] += a_ik * b[k * n + j];
            }
        }
    });
    result
}

#[pyfunction]
fn fast_string_join(parts: Vec<String>) -> String {
    parts.concat()
}

#[pyfunction]
fn fast_collect(nums: Vec<i64>, threshold: i64) -> Vec<i64> {
    nums.into_par_iter().filter(|&x| x > threshold).collect()
}

#[pyfunction]
fn fast_word_count(words: Vec<String>) -> HashMap<String, u64> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for w in words {
        *counts.entry(w).or_insert(0) += 1;
    }
    counts
}

// ===== Step 8: Parallel worker pool / "Cart Effect" ops =====

#[pyfunction]
#[pyo3(signature = (numbers, chunk_size=10000))]
fn batch_sum_of_squares(numbers: Vec<i64>, chunk_size: usize) -> i64 {
    numbers
        .par_chunks(chunk_size)
        .map(|c| c.iter().map(|&x| x * x).sum::<i64>())
        .sum()
}

#[pyfunction]
fn parallel_transform(data: Vec<f64>, operation: &str) -> Vec<f64> {
    data.par_iter()
        .map(|&x| match operation {
            "square" => x * x,
            "sqrt" => x.sqrt(),
            "cube" => x * x * x,
            _ => x,
        })
        .collect()
}

#[pyfunction]
fn get_worker_info() -> (usize, usize) {
    let physical = num_cpus::get_physical();
    let logical = num_cpus::get();
    (physical, logical)
}

#[pyfunction]
fn batch_process_with_progress(data: Vec<i64>, py: Python<'_>) -> PyResult<i64> {
    let result: i64 = py.allow_threads(|| {
        data.par_iter().map(|&x| x * x).sum()
    });
    Ok(result)
}

#[pymodule]
fn fast_ops(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_of_squares, m)?)?;
    m.add_function(wrap_pyfunction!(fast_range_sum, m)?)?;
    m.add_function(wrap_pyfunction!(heavy_loop, m)?)?;
    m.add_function(wrap_pyfunction!(is_prime, m)?)?;
    m.add_function(wrap_pyfunction!(count_primes, m)?)?;
    m.add_function(wrap_pyfunction!(fibonacci, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_multiply, m)?)?;
    m.add_function(wrap_pyfunction!(fast_string_join, m)?)?;
    m.add_function(wrap_pyfunction!(fast_collect, m)?)?;
    m.add_function(wrap_pyfunction!(fast_word_count, m)?)?;
    m.add_function(wrap_pyfunction!(batch_sum_of_squares, m)?)?;
    m.add_function(wrap_pyfunction!(parallel_transform, m)?)?;
    m.add_function(wrap_pyfunction!(get_worker_info, m)?)?;
    m.add_function(wrap_pyfunction!(batch_process_with_progress, m)?)?;
    Ok(())
}