use std::fs;
use std::process::Command;
use anyhow::{Result, Context};
use colored::*;
use std::collections::HashMap;

pub fn run_live_benchmark(python_bin: &str, pattern_name: &str, n_hint: u64) -> Result<()> {
    println!("\n{}", "🔬 Running live benchmark (Python vs Rust/fast_ops)...".cyan().bold());

    let (py_code, rust_call) = match build_template(pattern_name, n_hint) {
        Some(t) => t,
        None => {
            println!("⚠️  No benchmark template for pattern '{}'", pattern_name);
            return Ok(());
        }
    };

    let script = format!(
r#"
import time
try:
    import fast_ops
    HAS_RUST = True
except ImportError:
    HAS_RUST = False

start = time.time()
{py_code}
py_time = time.time() - start
print(f"PYTHON_TIME={{py_time:.6f}}")

if HAS_RUST:
    start = time.time()
    rust_result = {rust_call}
    rust_time = time.time() - start
    print(f"RUST_TIME={{rust_time:.6f}}")
else:
    print("RUST_TIME=NA")
"#,
        py_code = py_code, rust_call = rust_call
    );

    let temp_path = std::env::temp_dir().join("accel_bench_temp.py");
    fs::write(&temp_path, script)?;

    let output = Command::new(python_bin).arg(&temp_path).output()
        .context("Failed to run benchmark script")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut py_time: Option<f64> = None;
    let mut rust_time: Option<f64> = None;

    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("PYTHON_TIME=") { py_time = v.parse().ok(); }
        if let Some(v) = line.strip_prefix("RUST_TIME=") {
            if v != "NA" { rust_time = v.parse().ok(); }
        }
    }

    match (py_time, rust_time) {
        (Some(pt), Some(rt)) if rt > 0.0 => {
            let speedup = pt / rt;
            println!("{}", "════════ BENCHMARK RESULT ════════".green().bold());
            println!("Pure Python  : {:.6}s", pt);
            println!("Rust (PyO3)  : {:.6}s", rt);
            println!("{}", format!("Speedup      : {:.1}x faster 🚀", speedup).yellow().bold());
            println!("{}", "═══════════════════════════════════".green().bold());
        }
        (Some(pt), None) => {
            println!("Pure Python  : {:.6}s", pt);
            println!("{}", "⚠️  fast_ops not installed. Run: cd fast_ops && maturin develop --release".red());
        }
        _ => println!("⚠️  Could not parse benchmark output:\n{}", stdout),
    }

    let _ = fs::remove_file(&temp_path);
    Ok(())
}

fn build_template(pattern_name: &str, n_hint: u64) -> Option<(String, String)> {
    match pattern_name {
        "sum_of_squares_loop" => Some((
            format!("total = 0\nfor i in range({n}):\n    total += i * i\n", n = n_hint),
            format!("fast_ops.sum_of_squares({})", n_hint),
        )),
        "range_sum_loop" => Some((
            format!("total = 0\nfor i in range({n}):\n    total += i\n", n = n_hint),
            format!("fast_ops.fast_range_sum({})", n_hint),
        )),
        "fibonacci" => Some((
            format!("a, b = 0, 1\nfor _ in range({n}):\n    a, b = b, a + b\n", n = n_hint),
            format!("fast_ops.fibonacci({})", n_hint),
        )),
        "nested_loop_matrix" => {
            let n = n_hint.min(150); // n^3 growth — capped for sane demo runtime
            Some((
                format!(
                    "n = {n}\na = [[1.0]*n for _ in range(n)]\nb = [[1.0]*n for _ in range(n)]\nresult = [[0.0]*n for _ in range(n)]\nfor i in range(n):\n    for j in range(n):\n        for k in range(n):\n            result[i][j] += a[i][k] * b[k][j]\n",
                    n = n
                ),
                format!("fast_ops.matrix_multiply([1.0]*({n}*{n}), [1.0]*({n}*{n}), {n})", n = n),
            ))
        }
        "string_concat_loop" => {
            let n = n_hint.min(50_000);
            Some((
                format!("result = ''\nfor i in range({n}):\n    result += str(i)\n", n = n),
                format!("fast_ops.fast_string_join([str(i) for i in range({})])", n),
            ))
        }
        _ => None,
    }
}

/// Step 8: "Cart Effect" — shows scaling behaviour as load increases
/// (No support / Rust single-thread / Rust + parallel worker pool)
pub fn run_load_scaling_test(python_bin: &str, pattern_name: &str) -> Result<()> {
    println!("\n{}", "📈 Load Scaling Test (No Support → Rust → Rust+Parallel)".cyan().bold());

    if pattern_name != "sum_of_squares_loop" {
        // fallback: run regular benchmark at 3 sizes if no parallel template exists
        for size in [1_000_000u64, 10_000_000, 50_000_000] {
            run_live_benchmark(python_bin, pattern_name, size)?;
        }
        return Ok(());
    }

    let sizes: Vec<u64> = vec![1_000_000, 10_000_000, 50_000_000];

    for size in sizes {
        run_scaling_step(python_bin, size)?;
    }

    println!("\n{}", "💡 Observation: 'Rust+Parallel' ka speedup ratio load barhne ke sath BADHTA hai — ye 'cart' effect hai.".magenta());
    Ok(())
}

fn run_scaling_step(python_bin: &str, n: u64) -> Result<()> {
    let script = format!(r#"
import time
import fast_ops

n = {n}
data = list(range(n))

start = time.time()
total = 0
for i in data:
    total += i * i
py_time = time.time() - start

start = time.time()
fast_ops.sum_of_squares(n)
single_time = time.time() - start

start = time.time()
fast_ops.batch_sum_of_squares(data, 50000)
parallel_time = time.time() - start

print(f"N={{n}}")
print(f"PYTHON={{py_time:.6f}}")
print(f"RUST_SINGLE={{single_time:.6f}}")
print(f"RUST_PARALLEL={{parallel_time:.6f}}")
"#, n = n);

    let temp_path = std::env::temp_dir().join("accel_scaling_temp.py");
    fs::write(&temp_path, script)?;

    let output = Command::new(python_bin).arg(&temp_path).output()
        .context("Failed to run scaling test script")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut vals: HashMap<String, String> = HashMap::new();
    for line in stdout.lines() {
        if let Some((k, v)) = line.split_once('=') {
            vals.insert(k.to_string(), v.to_string());
        }
    }

    if let (Some(py), Some(single), Some(par)) = (
        vals.get("PYTHON").and_then(|v| v.parse::<f64>().ok()),
        vals.get("RUST_SINGLE").and_then(|v| v.parse::<f64>().ok()),
        vals.get("RUST_PARALLEL").and_then(|v| v.parse::<f64>().ok()),
    ) {
        println!("\n  n = {:>12}", n);
        println!("  🚶 Python         : {:.4}s", py);
        println!("  🦯 Rust (single)  : {:.4}s  ({:.1}x)", single, py / single.max(0.000001));
        println!("  🛒 Rust (parallel): {:.4}s  ({:.1}x) ← cart effect", par, py / par.max(0.000001));
    } else {
        println!("⚠️  Could not parse scaling test output for n={}:\n{}", n, stdout);
    }

    let _ = fs::remove_file(&temp_path);
    Ok(())
}