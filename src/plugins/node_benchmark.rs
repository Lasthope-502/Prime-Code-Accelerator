use std::fs;
use std::process::Command;
use anyhow::{Result, Context};
use colored::*;
use std::collections::HashMap;

fn fast_ops_path() -> Result<String> {
    let path = std::env::current_dir()?.join("fast_ops_node").join("index.js");
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn build_template(pattern_name: &str, n_hint: u64) -> Option<(String, String)> {
    match pattern_name {
        "sum_of_squares_loop" => Some((
            format!("let total = 0;\nfor (let i = 0; i < {n}; i++) {{ total += i * i; }}", n = n_hint),
            format!("fastOps.sumOfSquares({})", n_hint),
        )),
        "range_sum_loop" => Some((
            format!("let total = 0;\nfor (let i = 0; i < {n}; i++) {{ total += i; }}", n = n_hint),
            format!("fastOps.fastRangeSum({})", n_hint),
        )),
        _ => None,
    }
}

pub fn run_live_benchmark(node_bin: &str, pattern_name: &str, n_hint: u64) -> Result<()> {
    println!("\n{}", "🔬 Running live benchmark (JS vs Rust/native addon)...".cyan().bold());

    let (js_code, native_call) = match build_template(pattern_name, n_hint) {
        Some(t) => t,
        None => {
            println!("⚠️  No benchmark template for pattern '{}'", pattern_name);
            return Ok(());
        }
    };

    let fast_ops_path_str = fast_ops_path()?;

    let script = format!(
r#"
let fastOps = null;
try {{ fastOps = require('{fast_ops_path}'); }} catch (e) {{}}

let start = process.hrtime.bigint();
{js_code}
let jsTime = Number(process.hrtime.bigint() - start) / 1e9;
console.log("JS_TIME=" + jsTime.toFixed(6));

if (fastOps) {{
  start = process.hrtime.bigint();
  const result = {native_call};
  let nativeTime = Number(process.hrtime.bigint() - start) / 1e9;
  console.log("NATIVE_TIME=" + nativeTime.toFixed(6));
}} else {{
  console.log("NATIVE_TIME=NA");
}}
"#,
        fast_ops_path = fast_ops_path_str, js_code = js_code, native_call = native_call
    );

    let temp_path = std::env::temp_dir().join("accel_node_bench_temp.js");
    fs::write(&temp_path, script)?;

    let output = Command::new(node_bin).arg(&temp_path).output()
        .context("Failed to run node benchmark script")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut js_time: Option<f64> = None;
    let mut native_time: Option<f64> = None;

    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("JS_TIME=") { js_time = v.parse().ok(); }
        if let Some(v) = line.strip_prefix("NATIVE_TIME=") {
            if v != "NA" { native_time = v.parse().ok(); }
        }
    }

    match (js_time, native_time) {
        (Some(jt), Some(nt)) if nt > 0.0 => {
            let speedup = jt / nt;
            println!("{}", "════════ BENCHMARK RESULT ════════".green().bold());
            println!("Pure JS      : {:.6}s", jt);
            println!("Rust (napi)  : {:.6}s", nt);
            println!("{}", format!("Speedup      : {:.1}x faster 🚀", speedup).yellow().bold());
            println!("{}", "═══════════════════════════════════".green().bold());
        }
        (Some(jt), None) => {
            println!("Pure JS      : {:.6}s", jt);
            println!("{}", "⚠️  fast_ops_node not built. Run: cd fast_ops_node && npm install && npm run build".red());
        }
        _ => println!("⚠️  Could not parse benchmark output:\n{}", stdout),
    }

    let _ = fs::remove_file(&temp_path);
    Ok(())
}

/// Step 8: "Cart Effect" scaling test for Node.js
pub fn run_load_scaling_test(node_bin: &str, pattern_name: &str) -> Result<()> {
    println!("\n{}", "📈 Load Scaling Test (No Support → Rust → Rust+Parallel)".cyan().bold());

    if pattern_name != "sum_of_squares_loop" {
        for size in [1_000_000u64, 10_000_000, 50_000_000] {
            run_live_benchmark(node_bin, pattern_name, size)?;
        }
        return Ok(());
    }

    for size in [1_000_000u64, 10_000_000, 50_000_000] {
        run_scaling_step(node_bin, size)?;
    }

    println!("\n{}", "💡 Observation: 'Rust+Parallel' ka speedup ratio load barhne ke sath BADHTA hai — ye 'cart' effect hai.".magenta());
    Ok(())
}

fn run_scaling_step(node_bin: &str, n: u64) -> Result<()> {
    let fast_ops_path_str = fast_ops_path()?;

    let script = format!(r#"
let fastOps = null;
try {{ fastOps = require('{fast_ops_path}'); }} catch (e) {{}}

const n = {n};
const data = Array.from({{length: n}}, (_, i) => i);

let start = process.hrtime.bigint();
let total = 0;
for (let i = 0; i < n; i++) {{ total += i * i; }}
let jsTime = Number(process.hrtime.bigint() - start) / 1e9;

let singleTime = -1, parallelTime = -1;
if (fastOps) {{
  start = process.hrtime.bigint();
  fastOps.sumOfSquares(n);
  singleTime = Number(process.hrtime.bigint() - start) / 1e9;

  start = process.hrtime.bigint();
  fastOps.batchSumOfSquares(data);
  parallelTime = Number(process.hrtime.bigint() - start) / 1e9;
}}

console.log("N=" + n);
console.log("JS=" + jsTime.toFixed(6));
console.log("SINGLE=" + singleTime.toFixed(6));
console.log("PARALLEL=" + parallelTime.toFixed(6));
"#, fast_ops_path = fast_ops_path_str, n = n);

    let temp_path = std::env::temp_dir().join("accel_node_scaling_temp.js");
    fs::write(&temp_path, script)?;

    let output = Command::new(node_bin).arg(&temp_path).output()
        .context("Failed to run node scaling test")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut vals: HashMap<String, String> = HashMap::new();
    for line in stdout.lines() {
        if let Some((k, v)) = line.split_once('=') {
            vals.insert(k.to_string(), v.to_string());
        }
    }

    if let (Some(js), Some(single), Some(par)) = (
        vals.get("JS").and_then(|v| v.parse::<f64>().ok()),
        vals.get("SINGLE").and_then(|v| v.parse::<f64>().ok()),
        vals.get("PARALLEL").and_then(|v| v.parse::<f64>().ok()),
    ) {
        if single < 0.0 {
            println!("\n  n = {:>12} — ⚠️  fast_ops_node not built, skipping native comparison", n);
        } else {
            println!("\n  n = {:>12}", n);
            println!("  🚶 JS             : {:.4}s", js);
            println!("  🦯 Rust (single)  : {:.4}s  ({:.1}x)", single, js / single.max(0.000001));
            println!("  🛒 Rust (parallel): {:.4}s  ({:.1}x) ← cart effect", par, js / par.max(0.000001));
        }
    } else {
        println!("⚠️  Could not parse scaling test output for n={}:\n{}", n, stdout);
    }

    let _ = fs::remove_file(&temp_path);
    Ok(())
}