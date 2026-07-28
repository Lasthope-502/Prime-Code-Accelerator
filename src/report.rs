use serde::Serialize;
use std::time::Duration;
use std::fs::File;
use std::io::Write;
use anyhow::Result;
use colored::*;

#[derive(Clone, Serialize)]
pub struct Sample {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub timestamp_ms: u128,
}

#[derive(Serialize)]
pub struct Report {
    pub command: String,
    pub total_time_ms: u128,
    pub success: bool,
    pub peak_memory_mb: f64,
    pub avg_cpu_percent: f32,
    pub max_cpu_percent: f32,
    pub samples: Vec<Sample>,
}

impl Report {
    pub fn build(command: &str, elapsed: Duration, samples: &[Sample], success: bool) -> Self {
        let peak_mem_bytes = samples.iter().map(|s| s.memory_bytes).max().unwrap_or(0);
        let avg_cpu = if !samples.is_empty() {
            samples.iter().map(|s| s.cpu_percent).sum::<f32>() / samples.len() as f32
        } else {
            0.0
        };
        let max_cpu = samples.iter().map(|s| s.cpu_percent).fold(0.0, f32::max);

        Report {
            command: command.to_string(),
            total_time_ms: elapsed.as_millis(),
            success,
            peak_memory_mb: peak_mem_bytes as f64 / (1024.0 * 1024.0), // bytes -> MB (correct conversion)
            avg_cpu_percent: avg_cpu,
            max_cpu_percent: max_cpu,
            samples: samples.to_vec(),
        }
    }

    pub fn print_terminal(&self) {
        println!("\n{}", "════════ PRIME ACCELERATOR REPORT ════════".cyan().bold());
        println!("Command      : {}", self.command);
        println!("Status       : {}", if self.success { "✅ Success".green() } else { "❌ Failed".red() });
        println!("Total Time   : {} ms", self.total_time_ms.to_string().yellow());
        println!("Peak Memory  : {:.2} MB", self.peak_memory_mb);
        println!("Avg CPU      : {:.2}%", self.avg_cpu_percent);
        println!("Max CPU      : {:.2}%", self.max_cpu_percent);
        println!("Samples      : {}", self.samples.len());
        println!("{}\n", "════════════════════════════════════════".cyan().bold());

        self.suggest();
    }

    fn suggest(&self) {
        println!("{}", "💡 Suggestions:".magenta().bold());
        if self.avg_cpu_percent > 80.0 {
            println!(" - High CPU usage detected. Consider parallelization or offloading heavy loops to Rust (FFI).");
        }
        if self.peak_memory_mb > 500.0 {
            println!(" - High memory usage. Check for memory leaks or large in-memory data structures.");
        }
        if self.total_time_ms > 5000 {
            println!(" - Execution is slow (>5s). Profile hot functions and consider caching repeated computations.");
        }
        if self.avg_cpu_percent < 80.0 && self.total_time_ms > 3000 {
            println!(" - Low CPU but slow execution → likely I/O bound (DB/network). Consider async or connection pooling.");
        }
        println!();
    }

    pub fn save_json(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        println!("📄 Full report saved to {}", path);
        Ok(())
    }
}