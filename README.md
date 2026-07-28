# 🚀 Prime Code Accelerator

**A Rust-powered universal performance profiler and native-code accelerator for multi-language projects.**

Prime Code Accelerator (working name: *Velocity Core*) is a Rust-based engine that attaches to any project — regardless of language — profiles its execution, detects performance bottlenecks, and where possible, offloads slow code to compiled Rust for real, measurable speedups.

It is **not** a magic "make everything fast" button. It is an honest, practical tool built on a simple realization: most real-world performance problems come from a small number of recurring patterns (loops, string concatenation, nested iterations, collection building), and these specific patterns *can* be reliably detected and accelerated using native code — while everything else gets useful diagnostic information.

---

## Table of Contents

1. [Core Idea](#core-idea)
2. [Why Rust](#why-rust)
3. [Why Not Just Use C++](#why-not-just-use-c)
4. [Architecture Overview](#architecture-overview)
5. [How It Actually Works](#how-it-actually-works)
6. [The "Cart Effect" — Parallel Scaling](#the-cart-effect--parallel-scaling)
7. [Supported Languages](#supported-languages)
8. [Installation](#installation)
9. [Usage](#usage)
10. [Command Reference](#command-reference)
11. [Configuration (accel.toml)](#configuration-accel-toml)
12. [Available Optimization Patterns](#available-optimization-patterns)
13. [Project Structure](#project-structure)
14. [How Each Component Works](#how-each-component-works)
15. [Limitations & Honest Scope](#limitations--honest-scope)
16. [Roadmap](#roadmap)
17. [Contributing](#contributing)
18. [License](#license)

---

## Core Idea

Interpreted and JIT-compiled languages (Python, JavaScript, Ruby, PHP, Java, C#, Kotlin, Go) are almost always slower than compiled, ahead-of-time (AOT) languages like Rust or C++ for CPU-bound work. This is not a bug — it's a fundamental tradeoff these languages make in exchange for developer productivity, safety, and flexibility.

Instead of trying to make Python "magically" as fast as Rust (which is impossible without changing the language itself), Prime Code Accelerator takes a different, realistic approach:

1. **Profile** the actual running program to find *where* time is being spent (not guesses — real measured data).
2. **Detect** whether the slow part matches a known, common performance-killing pattern.
3. **Offload** that specific pattern to a pre-compiled Rust module via a language-native FFI bridge (PyO3 for Python, napi-rs for Node.js).
4. **Prove it** — run both versions live and show the actual before/after numbers, not theoretical claims.
5. **Suggest** improvements for everything else that can't be automatically offloaded (caching, vectorization, algorithm changes).

This means the tool gives **two kinds of value**:
- **Real, measured speedups** for detected patterns (via native Rust execution)
- **Actionable suggestions** for everything else (via profiling data)

---

## Why Rust

The core engine is written entirely in Rust because:

- **AOT-compiled**: Rust compiles to native machine code — no runtime interpreter overhead, no JIT warm-up delay. The tool itself needs to be fast since it's measuring/managing other programs.
- **Memory safety without garbage collection**: No GC pauses interfering with precise timing measurements.
- **Best-in-class FFI story**: Rust has mature, well-supported bridges to almost every language:
  - `PyO3` → Python
  - `napi-rs` → Node.js
  - `jni` → Java/Kotlin
  - Native C ABI → C, C++, Go, C#
- **`rayon` for effortless data parallelism**: Turning a sequential loop into a multi-core parallel one is often a one-line change in Rust.
- **Single static binary distribution**: A compiled Rust CLI can be shipped as one file with no runtime dependencies, unlike a Python or Node-based tool that would need its own runtime installed.

## Why Not Just Use C++

C++ was seriously considered for the native offload layer, since it's traditionally the fastest option and has the most mature ecosystem for performance work. It was set aside for the **first version** because:

- C++ has no built-in memory safety — bugs in the native module could crash or corrupt the *host* language's process (Python/Node), which is unacceptable for a tool meant to be trustworthy.
- C++ build tooling is fragmented and painful across platforms (CMake configs, ABI issues, compiler differences) — this conflicts with the goal of a simple, one-command install experience.
- Rust gives ~95% of C++'s raw performance for these workloads (loops, math, string processing) while being dramatically safer and easier to build cross-platform.

**However, this is not a permanent decision.** The architecture is designed so that a C++ backend could be added *alongside* Rust in the future for very specific, extreme-performance use cases (SIMD-heavy numerical kernels, GPU interop, etc.), while Rust continues to handle the safe, general-purpose native offload work. Rust and C++ can coexist in the same process without conflict since both compile to native code with a compatible C ABI.

---

## Architecture Overview
```bash

                Target Project (any language)
                          │
                          ▼
              ┌───────────────────────┐
              │   Rust Core Engine     │   (this repository's CLI)
              │  (CLI, orchestration)  │
              └───────────┬───────────┘
                          │
             ┌────────────┼────────────┐
             ▼            ▼             ▼
      ┌───────────┐ ┌───────────┐ ┌───────────┐
      │  Profiler  │ │  Language  │ │  Reporter  │
      │ (CPU/RAM/  │ │  Adapter   │ │ (terminal +│
      │  time)     │ │  (plugin)  │ │  JSON)     │
      └─────┬─────┘ └─────┬─────┘ └───────────┘
            │              │
            │              ▼
            │      ┌───────────────┐
            │      │ Pattern Match  │
            │      │ (AST / regex)  │
            │      └───────┬───────┘
            │              │
            │              ▼
            │      ┌───────────────┐
            │      │  Native FFI    │
            │      │  Offload       │
            │      │ (PyO3/napi-rs) │
            │      └───────┬───────┘
            │              │
            └──────┬───────┘
                   ▼
          Optimized Execution +
          Speedup Report
```

### The Four Layers

**Layer 1 — Core Engine (Rust)**
The brain of the tool. Handles CLI parsing, process spawning, orchestration between all other layers, and configuration management.

**Layer 2 — Dynamic Profiler**
Attaches to the running target process and samples CPU usage, memory consumption, and wall-clock time at regular intervals (default: every 100ms), completely independent of the target's language.

**Layer 3 — Language Adapters / Plugins**
Language-specific logic. For Python, this means using `cProfile` for function-level hotspot data and Python's own `ast` module for accurate pattern detection. For Node.js, this means using the built-in V8 Inspector Profiler API.

**Layer 4 — Native FFI + Offload Engine**
Where the actual speedup happens. Detected patterns are matched against a library of pre-written, pre-compiled Rust functions, callable directly from the host language via `PyO3` (Python) or `napi-rs` (Node.js), using `rayon` internally for automatic multi-core parallelism.

---

## How It Actually Works

### Step-by-step execution flow (Python example):

1. User runs: `accel run --accelerate -- python3 my_script.py`
2. The Rust CLI detects the language is Python (`lang_detect.rs`)
3. Instead of running `my_script.py` directly, it wraps it: `python3 <embedded_wrapper.py> my_script.py`
4. The wrapper script runs the target under Python's built-in `cProfile`, capturing per-function timing data
5. Meanwhile, the Rust engine spawns a background thread that samples the process's CPU% and memory usage every 100ms using the `sysinfo` crate
6. Once execution finishes, `cProfile` results are written to a JSON file
7. Rust reads this JSON and identifies the top time-consuming function (the "hotspot")
8. Rust calls a second embedded Python script that parses the hotspot function's source code into an **Abstract Syntax Tree (AST)** — not just text matching, actual syntax structure — and checks it against known slow patterns (e.g., "for loop with `total += i * i`")
9. If a pattern matches, Rust suggests the equivalent pre-built Rust function (e.g., `fast_ops.sum_of_squares()`)
10. If `--accelerate` is passed, Rust generates a small benchmark script that runs **both** the original Python code and the Rust-native equivalent, times both, and prints the real speedup ratio
11. Everything is also saved to `accel_report.json` for later analysis or CI integration

### Why AST instead of plain text/regex matching (Python)?

Early versions used regular expressions to scan source code for slow patterns. This is fragile — code with different variable names, spacing, or formatting could be missed, and comments or strings containing similar text could cause false positives.

By using Python's own `ast` module, the tool parses code the exact same way the Python interpreter does — analyzing actual structure (loop types, operators, variable bindings) rather than surface text. This makes detection reliable regardless of coding style.

Node.js pattern detection currently still uses regex (a full JS AST parser like `acorn` is a planned upgrade — see Roadmap).

---

## The "Cart Effect" — Parallel Scaling

A key design insight (and the origin of this feature): a single native function call gives a fixed speedup, but as data size grows, even a fast single-threaded Rust function eventually gets slower proportionally. Real, sustained performance under increasing load requires **parallelism**, not just "faster single execution."

Think of it like a person walking on unsupported stairs versus:
- **No support** = pure interpreted language execution (slowest, degrades badly with more load)
- **One support rail** = a single Rust FFI call (fast, but single-threaded — still slows down as load increases)
- **Both support rails + a cart** = Rust FFI *combined with* a parallel worker pool (via `rayon`) — as load increases, more CPU cores automatically get engaged, so the speedup ratio actually *increases* with load instead of staying flat or degrading

This is implemented via the `--scaling-test` flag, which runs the same workload at increasing sizes (1M → 10M → 50M items) and shows all three tiers side by side, proving this scaling behavior with real numbers rather than theory.
---
n = 1,000,000
Python : 0.1200s
Rust (single) : 0.0080s (15.0x)
Rust (parallel) : 0.0025s (48.0x) ← cart effect

n = 50,000,000
Python : 6.5000s
Rust (single) : 0.3800s (17.1x)
Rust (parallel) : 0.0450s (144.4x) ← cart effect grows stronger with load
---

Note: **Go was deliberately not used** for this parallel layer, despite Go being popular for concurrency. Rust's `rayon` (data parallelism) and `tokio` (async I/O) provide equal or superior concurrency capabilities within the same language as the rest of the native offload engine, avoiding the complexity of adding a third language and a fragile cross-language bridge (Rust↔Go FFI via cgo is notoriously painful) for no measurable benefit.

---

## Supported Languages

| Language | Profiling Method | Pattern Detection | Native Offload Bridge | Status |
|----------|-------------------|--------------------|-----------------------|--------|
| Python   | `cProfile` (built-in) | AST-based (Python `ast` module) | PyO3 | ✅ Full support |
| Node.js  | V8 Inspector Profiler (built-in) | Regex-based | napi-rs | ✅ Full support |
| Java/Kotlin | — | — | — | 🔜 Planned |
| Go | — | — | — | 🔜 Planned |
| C# | — | — | — | 🔜 Planned |
| PHP | — | — | — | 🔜 Planned |
| Ruby | — | — | — | 🔜 Planned |
| Anything else (any executable) | Generic time/CPU/memory only | ❌ | ❌ | ✅ Basic profiling works universally |

Even for unsupported languages, the core profiler (time, CPU%, memory) works on **any** executable or script, since it operates at the OS process level, not the language level.

---

## Installation & Setup (Windows/PowerShell)

### Prerequisites (Install These First)

| Requirement | Why Needed | Install Command / Link |
|---|---|---|
| Rust (rustup) | Compiles the core CLI engine | https://rustup.rs |
| Visual Studio Build Tools | Required by Rust to link on Windows (C++ linker) | https://visualstudio.microsoft.com/visual-cpp-build-tools/ (select "Desktop development with C++") |
| Python 3.8+ | Needed only if profiling Python projects | https://python.org |
| Node.js 16+ | Needed only if profiling Node.js projects | https://nodejs.org |
| maturin | Builds the Rust↔Python native bridge | `pip install maturin` |

Verify installations:

```powershell
cargo --version
rustc --version
python --version
node --version
```
### Step 1 — Build the Core CLI

Run this inside the project root folder:
```powershell
cargo build --release
```
This takes 1-3 minutes on first run (downloads dependencies). On success, the binary will be created at:
```text
target\release\prime-accelerator.exe
```
### Step 2 — Test the Binary Directly (without installing)

```PowerShell
.\target\release\prime-accelerator.exe --help
```
If this prints help text, the core engine built successfully.

### Step 3 — Install as a Global accel Command

```PowerShell
# Create install directory
mkdir "$env:USERPROFILE\.accel\bin" -Force

# Copy and rename the binary
Copy-Item ".\target\release\prime-accelerator.exe" "$env:USERPROFILE\.accel\bin\accel.exe"

# Add to PATH permanently (User scope)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";$env:USERPROFILE\.accel\bin", "User")
```
***Important***: Close and reopen PowerShell (or restart your terminal) after this step for the PATH change to take effect. Then verify:

```PowerShell
accel --help
```

If `accel` is recognized, installation is complete.

### Step 4 — Enable Python Support (fast_ops native module)

```PowerShell
pip install maturin
cd fast_ops
maturin develop --release
cd ..
```
If `maturin develop` fails with a virtual environment error, create one first:

```PowerShell
python -m venv venv
.\venv\Scripts\Activate.ps1
pip install maturin
cd fast_ops
maturin develop --release
cd ..
```

Note: Every time you activate a new terminal session for Python profiling, you must re-activate the venv:

```PowerShell
.\venv\Scripts\Activate.ps1
```

### Step 5 — Enable Node.js Support (fast_ops_node native addon)

```PowerShell
cd fast_ops_node
npm install
npm run build
cd ..
```
---

## Installation

### Prerequisites
- [Rust](https://rustup.rs) (stable toolchain)
- Python 3.8+ (only needed if profiling Python projects)
- Node.js 16+ (only needed if profiling Node.js projects)
- [maturin](https://github.com/PyO3/maturin) (only needed for building the Python native module) — installed automatically by the install script
- npm (only needed for building the Node native addon)

### Linux / macOS

```bash
git clone <https://github.com/Lasthope-502/Prime-Code-Accelerator>
cd prime-accelerator
chmod +x install.sh
./install.sh
```
## Windows(PowerShell)
```bash
git clone <https://github.com/Lasthope-502/Prime-Code-Accelerator>
cd prime-accelerator
.\install.ps1
```
The install script will:

Build the Rust CLI in release mode
Install it as accel in your local bin directory
Optionally build the Python native module (fast_ops)
Optionally build the Node.js native addon (fast_ops_node)

**Manual Build (any platform)**
```bash
# 1. Build the core CLI
cargo build --release

# 2. Build the Python native extension
cd fast_ops
pip install maturin
maturin develop --release
cd ..

# 3. Build the Node native addon
cd fast_ops_node
npm install
npm run build
cd ..

# 4. The compiled binary is at:
./target/release/prime-accelerator
```
---

## Usage
```bash
# Basic profiling — works for literally any language/executable
accel run -- python3 script.py
accel run -- node app.js
accel run -- ruby script.rb
accel run -- ./my_compiled_binary

# Profile + show live Rust speedup benchmark for detected patterns
accel run --accelerate -- python3 script.py

# Profile + show how speedup scales with increasing data size (the "cart effect")
accel run --scaling-test -- python3 script.py

# Generate a default config file in the current project
accel init

# List all currently supported optimization patterns
accel patterns
```
---

## Example Output
```bash
🚀 Prime Accelerator: Starting profiling...

════════ PRIME ACCELERATOR REPORT ════════
Command      : python3 test_slow.py
Status       : ✅ Success
Total Time   : 4521 ms
Peak Memory  : 245.30 MB
Avg CPU      : 87.5%
Max CPU      : 99.2%
Samples      : 45
════════════════════════════════════════

💡 Suggestions:
 - High CPU usage detected. Consider parallelization or offloading heavy loops to Rust (FFI).

🔥 PYTHON HOT FUNCTIONS (Top 10 by cumulative time)
Function                            Calls     Total(s)  Cumulative(s)
----------------------------------------------------------------------
slow_function                           1       1.8500         1.8500

🎯 Pattern(s) Detected (AST-verified)!
 - [numeric_loop] 'total * total' accumulation pattern (AST-verified) (line 4)
   → Suggested Rust function: fast_ops.sum_of_squares()

🔬 Running live benchmark (Python vs Rust/fast_ops)...
════════ BENCHMARK RESULT ════════
Pure Python  : 1.850000s
Rust (PyO3)  : 0.021000s
Speedup      : 88.1x faster 🚀
═══════════════════════════════════

📄 Full report saved to accel_report.json
```
---

# Accel – Command Reference & Configuration

Accel is a performance profiling and optimization tool that detects slow patterns in your code and replaces them with native, high‑speed implementations.

---

## 📋 Command Reference

| Command | Description |
|---------|-------------|
| `accel run -- <command> [args...]` | Profile any command or script. |
| `accel run --accelerate -- <command>` | Run a **live native‑vs‑original speedup benchmark**. |
| `accel run --scaling-test -- <command>` | Show speedup behavior across increasing data sizes. |
| `accel init` | Create a default `accel.toml` config file in the current directory. |
| `accel patterns` | List all available optimization patterns for supported languages. |
| `accel --version` | Show version. |
| `accel --help` | Show help. |

---

## ⚙️ Configuration (`accel.toml`)

Run `accel init` to generate this file in your project root:

```toml
[general]
language = "auto"          # "auto", "python", or "node"
auto_accelerate = false    # always run --accelerate behavior by default

[profiler]
sample_interval_ms = 100   # how often to sample CPU/memory
report_output = "accel_report.json"

[thresholds]
high_cpu_percent = 80.0
high_memory_mb = 500.0
slow_execution_ms = 5000

[python]
binary = ""                 # leave blank to auto-detect from PATH
fast_ops_path = ""

[node]
binary = ""
fast_ops_path = "./fast_ops_node"

[patterns]
custom_patterns_file = ""
```
---

## Project Structure

```bash
prime-accelerator/
├── Cargo.toml
├── accel.toml.example
├── install.sh
├── install.ps1
├── README.md
│
├── src/
│   ├── main.rs                # CLI entry, argument parsing, command routing
│   ├── config.rs               # accel.toml loader
│   ├── profiler.rs             # Universal time/CPU/memory profiler
│   ├── report.rs                # Report struct + terminal/JSON output
│   ├── lang_detect.rs           # Language detection from command
│   └── plugins/
│       ├── mod.rs
│       ├── python.rs             # Python profiling orchestration
│       ├── ast_matcher.rs        # Bridges to Python AST analyzer
│       ├── benchmark.rs          # Python live benchmark + scaling test
│       ├── node.rs                # Node.js profiling orchestration
│       ├── node_pattern.rs        # Regex pattern matching for JS
│       └── node_benchmark.rs      # Node live benchmark + scaling test
│
├── scripts/
│   ├── py_profile_wrapper.py     # cProfile-based hotspot extractor
│   ├── ast_analyzer.py            # AST-based pattern detector
│   └── node_profile_wrapper.js    # V8 Inspector profiler wrapper
│
├── fast_ops/                     # Rust → Python native module (PyO3)
│   ├── Cargo.toml
│   ├── pyproject.toml
│   └── src/lib.rs
│
└── fast_ops_node/                # Rust → Node.js native addon (napi-rs)
    ├── Cargo.toml
    ├── build.rs
    ├── package.json
    └── src/lib.rs
```
---

## How Each Component Works

`src/profiler.rs`
Spawns the target process, then starts a background thread using the `sysinfo` crate to poll the process's PID every 100ms, recording CPU usage percentage and memory (RSS) in kilobytes. This works identically regardless of what language the target process is written in — it's purely OS-level process monitoring.

`src/plugins/python.rs`
Wraps the target Python script with an embedded `cProfile`-based wrapper (`scripts/py_profile_wrapper.py`), extracts per-function call counts and cumulative time, identifies the biggest bottleneck function, and hands its source location off to the AST matcher.

`scripts/ast_analyzer.py`
Uses Python's built-in `ast` module to parse the identified hotspot function into a syntax tree, then walks the tree looking for specific node patterns (e.g., an `AugAssign` node with an `Add` operator whose value is a `BinOp` multiplication of the loop variable by itself — i.e., `total += i * i`). This structural matching is far more reliable than text/regex matching.

`src/plugins/benchmark.rs`
When `--accelerate` is passed, generates a temporary Python script containing both the original slow pattern and a call to the equivalent pre-compiled Rust function, executes it, parses the timing output, and computes the real speedup ratio.

`fast_ops/src/lib.rs`
A separate Rust crate compiled via `maturin` into a Python-importable native module (`.so` on Linux, `.pyd` on Windows). Functions are exposed to Python using `#[pyfunction]` macros from PyO3. Many use `rayon's` parallel iterators (`par_iter`, `par_chunks`) to automatically distribute work across all available CPU cores.

`src/plugins/node.rs` **and** `scripts/node_profile_wrapper.js`
Uses Node's built-in `inspector` module to start the V8 CPU Profiler programmatically, run the target script, then stop the profiler and aggregate hit-count data per function — similar in spirit to `cProfile` but using V8's native profiling infrastructure.

`fast_ops_node/src/lib.rs`
A Rust crate compiled via `napi-rs` into a native Node.js addon (`.node` file), exposing the same style of parallel, high-performance functions to JavaScript using `#[napi]` macros.

---

## Limitations & Honest Scope

This section exists because it's important to be transparent about what this tool cannot do, so expectations are set correctly:

- It cannot automatically translate arbitrary code into Rust. True universal code translation across languages is an open research problem requiring deep program analysis or AI-assisted translation. This tool matches against a curated library of common, well-understood slow patterns — currently ~6-8 per language.
- Pattern detection has false negatives. If your slow code doesn't structurally match one of the known patterns, the tool will still show you profiling data (where the time is going) but won't be able to offer an automatic native offload.
- Node.js pattern matching is regex-based, not AST-based (unlike Python). This is less robust to unusual code formatting. Upgrading this to a proper JS AST parser (e.g., via the acorn npm package or a Rust-native JS parser like swc) is a planned improvement.
- Async/deferred code in Node.js may not be fully captured by the current profiler wrapper, since it stops profiling shortly after the main synchronous execution completes. Long-running promises or timers scheduled far in the future may be missed.
- The benchmark comparisons use synthetic re-execution of the detected pattern (not literally hot-swapping your running code) — this is intentional and safe (it never modifies your source files), but it means the benchmark measures the pattern, not necessarily every side effect of your specific implementation.
- This is an early-stage MVP. It has been tested primarily on numeric loops, string operations, and basic collection patterns. Production hardening, edge-case handling, and broader real-world validation are ongoing.
- The core philosophy: be honest about what's a measured fact (profiling data, benchmark numbers) versus what's a heuristic suggestion, and never claim a speedup that hasn't been actually measured on the user's machine.

---

## Roadmap

-  JVM support (Java/Kotlin) via jni crate + GC tuning suggestions
-  Go and C# basic profiling support
-  Replace Node.js regex pattern matching with true AST-based detection (via acorn or swc)
-  Web-based HTML dashboard for visualizing reports (instead of terminal-only output)
-  Expandable pattern library loaded from external JSON/TOML files (no recompilation needed to add patterns)
-  Optional C++ backend for extreme-performance SIMD/GPU-adjacent workloads, running alongside the Rust engine
-  CI/CD integration mode (fail a build if performance regresses beyond a threshold)
-  Caching layer for repeated expensive computations across runs

---

## Contributing

This project is intentionally modular:

- New language support = new file in `src/plugins/`
- New optimization pattern = new detection rule + corresponding Rust function in `fast_ops` or `fast_ops_node`
Pull requests, pattern suggestions, and bug reports are welcome.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---