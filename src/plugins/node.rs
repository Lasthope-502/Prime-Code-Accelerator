use std::fs;
use anyhow::{Result, Context};
use serde::Deserialize;
use colored::*;
use crate::profiler;
use crate::plugins::node_pattern;
use crate::plugins::node_benchmark;

const WRAPPER_SCRIPT: &str = include_str!("../../scripts/node_profile_wrapper.js");
const PROFILE_OUTPUT: &str = "accel_node_profile.json";

#[derive(Deserialize, Clone)]
pub struct NodeHotspot {
    pub function: String,
    pub filename: String,
    pub line: u64,
    pub hit_count: u64,
    pub estimated_self_time_ms: f64,
}

pub fn run_with_profiling(original_cmd: &[String], accelerate: bool, scaling_test: bool) -> Result<()> {
    let mut wrapper_path = std::env::temp_dir();
    wrapper_path.push("accel_node_wrapper.js");
    fs::write(&wrapper_path, WRAPPER_SCRIPT)
        .context("Failed to write node wrapper script")?;

    let _ = fs::remove_file(PROFILE_OUTPUT);

    let mut new_cmd = vec![original_cmd[0].clone(), wrapper_path.to_string_lossy().to_string()];
    new_cmd.extend_from_slice(&original_cmd[1..]);

    let report = profiler::run_and_profile(&new_cmd)?;
    report.print_terminal();

    match load_hotspots(PROFILE_OUTPUT) {
        Ok(hotspots) => {
            print_hotspots(&hotspots);

            if let Some(top) = hotspots.first() {
                let matches = node_pattern::detect_patterns(&top.filename, top.line)?;
                if !matches.is_empty() {
                    println!("{}", "🎯 Pattern(s) Detected!".green().bold());
                    for m in &matches {
                        println!(" - [{}] {}", m.category, m.description);
                        println!("   → Suggested native addon function: fastOps.{}()", m.rust_fn);
                    }
                    println!();

                    if scaling_test {
                        node_benchmark::run_load_scaling_test(&original_cmd[0], &matches[0].name)?;
                    } else if accelerate {
                        let n_hint = if top.hit_count > 1 { top.hit_count * 1000 } else { 10_000_000 };
                        node_benchmark::run_live_benchmark(&original_cmd[0], &matches[0].name, n_hint)?;
                    } else {
                        println!("💡 Tip: run with `--accelerate` or `--scaling-test` to see live JS vs Rust(native) speedup\n");
                    }
                } else {
                    println!("ℹ️  No known optimization pattern matched for top hotspot.\n");
                }
            }
        }
        Err(e) => println!("⚠️  Could not read Node profile data: {}", e),
    }

    report.save_json("accel_report.json")?;
    let _ = fs::remove_file(&wrapper_path);
    Ok(())
}

fn load_hotspots(path: &str) -> Result<Vec<NodeHotspot>> {
    let data = fs::read_to_string(path)
        .context("Profile JSON not found (did script run successfully?)")?;
    Ok(serde_json::from_str(&data)?)
}

fn print_hotspots(hotspots: &[NodeHotspot]) {
    println!("{}", "🔥 NODE.JS HOT FUNCTIONS (Top 10 by hit count)".red().bold());
    println!("{:<28} {:>10} {:>18}", "Function", "Hits", "Est. Self Time(ms)");
    println!("{}", "-".repeat(60));

    for h in hotspots.iter().take(10) {
        let name = if h.function.len() > 26 { format!("{}...", &h.function[..23]) } else { h.function.clone() };
        println!("{:<28} {:>10} {:>18.2}", name, h.hit_count, h.estimated_self_time_ms);
    }
    println!();
}