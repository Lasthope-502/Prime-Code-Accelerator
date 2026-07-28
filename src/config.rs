use serde::Deserialize;
use std::fs;
use std::path::Path;
use anyhow::Result;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub profiler: ProfilerConfig,
    #[serde(default)]
    pub thresholds: ThresholdsConfig,
    #[serde(default)]
    pub python: LangConfig,
    #[serde(default)]
    pub node: LangConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub auto_accelerate: bool,
}
fn default_language() -> String { "auto".to_string() }
impl Default for GeneralConfig {
    fn default() -> Self { Self { language: default_language(), auto_accelerate: false } }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProfilerConfig {
    #[serde(default = "default_interval")]
    pub sample_interval_ms: u64,
    #[serde(default = "default_report_path")]
    pub report_output: String,
}
fn default_interval() -> u64 { 100 }
fn default_report_path() -> String { "accel_report.json".to_string() }
impl Default for ProfilerConfig {
    fn default() -> Self {
        Self { sample_interval_ms: default_interval(), report_output: default_report_path() }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ThresholdsConfig {
    #[serde(default = "default_cpu")]
    pub high_cpu_percent: f32,
    #[serde(default = "default_mem")]
    pub high_memory_mb: f64,
    #[serde(default = "default_slow_ms")]
    pub slow_execution_ms: u128,
}
fn default_cpu() -> f32 { 80.0 }
fn default_mem() -> f64 { 500.0 }
fn default_slow_ms() -> u128 { 5000 }
impl Default for ThresholdsConfig {
    fn default() -> Self {
        Self { high_cpu_percent: default_cpu(), high_memory_mb: default_mem(), slow_execution_ms: default_slow_ms() }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LangConfig {
    #[serde(default)]
    pub binary: String,
    #[serde(default)]
    pub fast_ops_path: String,
}

impl Config {
    pub fn load() -> Self {
        let candidates = ["accel.toml", ".accel.toml"];
        for path in candidates {
            if Path::new(path).exists() {
                if let Ok(cfg) = Self::load_from(path) {
                    println!("📋 Loaded config from {}", path);
                    return cfg;
                }
            }
        }
        Config::default_config()
    }

    fn load_from(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }

    fn default_config() -> Self {
        Config {
            general: GeneralConfig::default(),
            profiler: ProfilerConfig::default(),
            thresholds: ThresholdsConfig::default(),
            python: LangConfig::default(),
            node: LangConfig::default(),
        }
    }
}