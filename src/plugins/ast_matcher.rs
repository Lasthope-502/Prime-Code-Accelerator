use std::fs;
use std::process::Command;
use anyhow::{Result, Context};
use serde::Deserialize;

const AST_ANALYZER_SCRIPT: &str = include_str!("../../scripts/ast_analyzer.py");

#[derive(Deserialize, Clone, Debug)]
pub struct PatternMatch {
    pub name: String,
    pub description: String,
    pub rust_fn: String,
    pub category: String,
    pub line: u64,
}

pub fn detect_patterns_ast(python_bin: &str, filename: &str, line: u64) -> Result<Vec<PatternMatch>> {
    let mut script_path = std::env::temp_dir();
    script_path.push("accel_ast_analyzer.py");
    fs::write(&script_path, AST_ANALYZER_SCRIPT)
        .context("Failed to write AST analyzer script")?;

    let output = Command::new(python_bin)
        .arg(&script_path)
        .arg(filename)
        .arg(line.to_string())
        .output()
        .context("Failed to run AST analyzer (is python3 in PATH?)")?;

    let _ = fs::remove_file(&script_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("AST analyzer failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let matches: Vec<PatternMatch> = serde_json::from_str(stdout.trim()).unwrap_or_default();

    Ok(matches)
}