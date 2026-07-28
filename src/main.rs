mod profiler;
mod report;
mod lang_detect;
mod config;
mod plugins;

use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;

#[derive(Parser)]
#[command(name = "accel")]
#[command(version = "0.1.0")]
#[command(about = "🚀 Prime Code Accelerator — Universal Performance Profiler & Optimizer", long_about = None)]
#[command(after_help = "EXAMPLES:\n  accel run -- python3 script.py\n  accel run --accelerate -- node app.js\n  accel run --scaling-test -- python3 script.py\n  accel init          # create accel.toml in current dir\n  accel patterns      # list available optimization patterns")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run and profile any script/program
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,

        /// Enable live Rust/native vs source-language speedup benchmark
        #[arg(long)]
        accelerate: bool,

        /// Show how speedup scales with increasing load (the "cart effect")
        #[arg(long)]
        scaling_test: bool,
    },
    /// Create a default accel.toml config file in current directory
    Init,
    /// List all available optimization patterns
    Patterns,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { cmd, accelerate, scaling_test } => {
            if cmd.is_empty() {
                eprintln!("{}", "❌ Usage: accel run -- <command> [args...]".red());
                eprintln!("   Example: accel run -- python3 script.py");
                std::process::exit(1);
            }

            let cfg = config::Config::load();
            let accelerate = accelerate || cfg.general.auto_accelerate;

            match lang_detect::detect(&cmd) {
                lang_detect::Language::Python => {
                    plugins::python::run_with_profiling(&cmd, accelerate, scaling_test)?;
                }
                lang_detect::Language::Node => {
                    plugins::node::run_with_profiling(&cmd, accelerate, scaling_test)?;
                }
                lang_detect::Language::Unknown => {
                    println!("{}", "ℹ️  Unrecognized language — running generic profiler only (no pattern detection/FFI offload available).".yellow());
                    let report = profiler::run_and_profile(&cmd)?;
                    report.print_terminal();
                    report.save_json(&cfg.profiler.report_output)?;
                }
            }
        }
        Commands::Init => {
            init_config()?;
        }
        Commands::Patterns => {
            list_patterns();
        }
    }

    Ok(())
}

fn init_config() -> Result<()> {
    let path = "accel.toml";
    if std::path::Path::new(path).exists() {
        println!("{}", "⚠️  accel.toml already exists in this directory.".yellow());
        return Ok(());
    }
    let template = include_str!("../accel.toml.example");
    std::fs::write(path, template)?;
    println!("{}", "✅ Created accel.toml — edit it to customize behavior.".green());
    Ok(())
}

fn list_patterns() {
    println!("{}", "📚 Available Optimization Patterns:\n".cyan().bold());
    println!("{}", "Python (AST-based):".yellow().bold());
    for p in [
        "sum_of_squares_loop", "range_sum_loop", "nested_loop_matrix",
        "string_concat_loop", "list_append_loop", "dict_counting_loop",
    ] {
        println!("  - {}", p);
    }
    println!("\n{}", "Node.js (regex-based):".yellow().bold());
    for p in [
        "sum_of_squares_loop", "range_sum_loop", "nested_loop_matrix",
        "string_concat_loop", "array_push_loop", "object_counting_loop",
    ] {
        println!("  - {}", p);
    }
    println!();
}