use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;
use sysinfo::{System, Pid, ProcessesToUpdate};
use anyhow::Result;
use crate::report::{Report, Sample};

pub fn run_and_profile(cmd_parts: &[String]) -> Result<Report> {
    println!("🚀 Prime Accelerator: Starting profiling...\n");

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let start = Instant::now();

    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let pid = Pid::from_u32(child.id());
    let samples: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = Arc::clone(&samples);

    let running = Arc::new(Mutex::new(true));
    let running_clone = Arc::clone(&running);

    let monitor_handle = thread::spawn(move || {
        let mut sys = System::new_all();
        while *running_clone.lock().unwrap() {
            sys.refresh_processes(ProcessesToUpdate::Some(&[pid]));
            if let Some(process) = sys.process(pid) {
                samples_clone.lock().unwrap().push(Sample {
                    cpu_percent: process.cpu_usage(),
                    memory_bytes: process.memory(), // sysinfo 0.31 returns BYTES, not KB
                    timestamp_ms: start.elapsed().as_millis(),
                });
            } else {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    let status = child.wait()?;
    *running.lock().unwrap() = false;
    let _ = monitor_handle.join();

    let elapsed = start.elapsed();
    let collected = samples.lock().unwrap().clone();

    Ok(Report::build(&cmd_parts.join(" "), elapsed, &collected, status.success()))
}