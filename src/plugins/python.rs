use std::fs;
use anyhow::{Result, Context};
use serde::Deserialize;
use colored::*;
use crate::profiler;
use crate::plugins::ast_matcher;
use crate::plugins::benchmark;

const WRAPPER_SCRIPT: &str = include_str!("../../scripts/py_profile_wrapper.py");
const PROFILE_OUTPUT: &str = "accel_py_profile.json";

#[derive(Deserialize, Clone)]
pub struct PyHotspot {
    pub filename: String,
    pub line: u64,
    pub function: String,
    pub calls: u64,
    pub total_time: f64,
    pub cumulative_time: f64,
}

pub fn run_with_profiling(original_cmd: &[String], accelerate: bool, scaling_test: bool) -> Result<()> {
    let mut wrapper_path = std::env::temp_dir();
    wrapper_path.push("accel_py_wrapper.py");
    fs::write(&wrapper_path, WRAPPER_SCRIPT)
        .context("Failed to write python wrapper script")?;

    let _ = fs::remove_file(PROFILE_OUTPUT);

    let mut new_cmd = vec![original_cmd[0].clone(), wrapper_path.to_string_lossy().to_string()];
    new_cmd.extend_from_slice(&original_cmd[1..]);

    let report = profiler::run_and_profile(&new_cmd)?;
    report.print_terminal();

    match load_hotspots(PROFILE_OUTPUT) {
        Ok(hotspots) => {
            print_hotspots(&hotspots, report.total_time_ms);

            let target_script = original_cmd.get(1).map(|s| s.as_str()).unwrap_or("");

            if let Some(top) = find_best_hotspot(&hotspots, target_script) {
                match ast_matcher::detect_patterns_ast(&original_cmd[0], &top.filename, top.line) {
                    Ok(matches) if !matches.is_empty() => {
                        println!("{}", "🎯 Pattern(s) Detected (AST-verified)!".green().bold());
                        for m in &matches {
                            println!(" - [{}] {} (line {})", m.category, m.description, m.line);
                            println!("   → Suggested Rust function: fast_ops.{}()", m.rust_fn);
                        }
                        println!();

                        if scaling_test {
                            benchmark::run_load_scaling_test(&original_cmd[0], &matches[0].name)?;
                        } else if accelerate {
                            let n_hint = if top.calls > 1 { top.calls * 100 } else { 10_000_000 };
                            benchmark::run_live_benchmark(&original_cmd[0], &matches[0].name, n_hint)?;
                        } else {
                            println!("💡 Tip: run with `--accelerate` or `--scaling-test` to see live Rust speedup\n");
                        }
                    }
                    Ok(_) => println!("ℹ️  No known optimization pattern matched (AST analysis) for function '{}'.\n", top.function),
                    Err(e) => println!("⚠️  AST analysis failed: {}\n", e),
                }
            } else {
                println!("ℹ️  Could not identify a clear user-code hotspot for pattern analysis.\n");
            }
        }
        Err(e) => println!("⚠️  Could not read Python profile data: {}", e),
    }

    report.save_json("accel_report.json")?;
    let _ = fs::remove_file(&wrapper_path);
    Ok(())
}

/// Finds the most relevant hotspot for pattern analysis:
/// - Must belong to the actual target script (not builtins, not the wrapper)
/// - Must not be a synthetic frame like <module>, <listcomp>, <genexpr>
/// - Picked by highest SELF time (total_time), not cumulative time,
///   since cumulative time is dominated by outer wrapper frames (exec, <module>, main)
fn find_best_hotspot<'a>(hotspots: &'a [PyHotspot], target_script: &str) -> Option<&'a PyHotspot> {
    let target_name = std::path::Path::new(target_script)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| target_script.to_string());

    if target_name.is_empty() {
        return None;
    }

    hotspots
        .iter()
        .filter(|h| {
            let h_name = std::path::Path::new(&h.filename)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| h.filename.clone());
            h_name == target_name && !h.function.starts_with('<')
        })
        .max_by(|a, b| {
            a.total_time
                .partial_cmp(&b.total_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn load_hotspots(path: &str) -> Result<Vec<PyHotspot>> {
    let data = fs::read_to_string(path)
        .context("Profile JSON not found (did script run successfully?)")?;
    let hotspots: Vec<PyHotspot> = serde_json::from_str(&data)?;
    Ok(hotspots)
}

fn print_hotspots(hotspots: &[PyHotspot], total_time_ms: u128) {
    println!("{}", "🔥 PYTHON HOT FUNCTIONS (Top 10 by cumulative time)".red().bold());
    println!("{:<30} {:>10} {:>12} {:>14}", "Function", "Calls", "Total(s)", "Cumulative(s)");
    println!("{}", "-".repeat(70));

    let total_sec = total_time_ms as f64 / 1000.0;

    for h in hotspots.iter().take(10) {
        let short_name = if h.function.len() > 28 {
            format!("{}...", &h.function[..25])
        } else {
            h.function.clone()
        };
        println!(
            "{:<30} {:>10} {:>12.4} {:>14.4}",
            short_name, h.calls, h.total_time, h.cumulative_time
        );
    }

    println!();
    println!("{}", "💡 Function-level Suggestions:".magenta().bold());

    for h in hotspots.iter().take(5) {
        let pct = if total_sec > 0.0 { (h.cumulative_time / total_sec) * 100.0 } else { 0.0 };

        if pct > 25.0 {
            println!(
                " ⚠️  '{}' consumes {:.1}% of total runtime ({} calls) — top candidate for Rust FFI offload.",
                h.function, pct, h.calls
            );
        } else if h.calls > 10_000 {
            println!(
                " - '{}' called {} times — consider @functools.lru_cache or vectorizing with NumPy.",
                h.function, h.calls
            );
        } else if h.total_time / (h.calls.max(1) as f64) > 0.01 {
            println!(
                " - '{}' has high per-call cost ({:.4}s/call) — check for I/O or blocking calls inside.",
                h.function, h.total_time / (h.calls.max(1) as f64)
            );
        }
    }
    println!();
}