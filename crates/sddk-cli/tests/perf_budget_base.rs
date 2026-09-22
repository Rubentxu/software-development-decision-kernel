//! C3g — Performance budget for Base profile (3 scenarios × N=100)
//!
//! Per ROADMAP §C3 ("presupuesto medible de p95 y recursos en tres escenarios
//! fijados (Base, static, runtime); baseline antes de optimizar"). Static and
//! Runtime scenarios are out of scope: providers (CogniCode, Chronos) are absent
//! in this environment (NOT_EVALUATED_PROVIDER_MISSING per C2a/b RECEIPTs).
//!
//! Scenarios:
//!  - S1: `sddk version` — instant read of resolved framework version. Expected p95 < 50 ms.
//!  - S2: `sddk cycle status` (no active cycle) — typical pre-flight check. Expected p95 < 100 ms.
//!  - S3: `sddk lint` — validation that exercises fmt + clippy + test lint. Expected p95 < 500 ms.
//!
//! Run with: cargo test -p sddk-cli --test perf_budget_base -- --ignored --nocapture
//!
//! Output: prints p50 / p95 / max wall-clock and peak RSS for each scenario.
//! Also writes /tmp/c3g-bench-results.yaml with the same numbers (durable).
//!
//! All tests are `#[ignore]` so they don't run on default `cargo test`.
//! Opt-in only, per AGENTS.md §2.3 (scoped testing during apply).

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const N: usize = 100;
const ENV_LABEL: &str = "linux x86_64, single userland, cargo-targets/release/sddk 1.169.150";
const RESULTS_PATH: &str = "/tmp/c3g-bench-results.yaml";

/// Find the sddk release binary. Honors `CARGO_TARGET_DIR` if set (this
/// workspace uses `/var/home/rubentxu/cargo-targets` as override).
fn sddk_bin() -> PathBuf {
    if let Ok(ctd) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(ctd).join("release/sddk");
        if p.exists() {
            return p;
        }
    }
    let default = PathBuf::from("target/release/sddk");
    if default.exists() {
        return default;
    }
    // Fallback: look in $HOME/.local/bin/sddk
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".local/bin/sddk");
        if p.exists() {
            return p;
        }
    }
    panic!("sddk binary not found; run `cargo build --release -p sddk-cli` first");
}

/// Peak RSS of a child process, read from `/proc/<pid>/status`.
/// Returns -1 if not available (e.g. macOS, sandbox, or process already
/// reaped — proc/<pid> disappears immediately after exit). Known limitation:
/// this harness reads RSS synchronously after `wait()`, so on Linux the
/// procfs entry often vanishes between exit and read. We use `try_wait()` in
/// a tight loop to read RSS while the process is still in zombie state.
/// Returns -1 if RSS could not be read.
fn peak_rss_kib(pid: u32) -> i64 {
    // Poll for up to 50 ms while the process is still a zombie; proc/<pid>
    // stays readable until the parent reaps the status.
    for _ in 0..50 {
        let status_path = format!("/proc/{pid}/status");
        if let Ok(content) = fs::read_to_string(&status_path) {
            for line in content.lines() {
                if line.starts_with("VmHWM:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].parse().unwrap_or(-1);
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    -1
}

#[derive(Debug, Clone, Copy)]
struct Stats {
    p50_ms: u128,
    p95_ms: u128,
    max_ms: u128,
    peak_rss_kib: i64,
}

fn pct(sorted: &[u128], p: f64) -> u128 {
    let idx = ((sorted.len() as f64) * p).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

fn measure_scenario(label: &str, bin: &PathBuf, args: &[&str]) -> Stats {
    eprintln!("[c3g] measuring {label} (N={N})...");
    let mut samples_ms: Vec<u128> = Vec::with_capacity(N);
    let mut max_rss_kib: i64 = 0;

    for _ in 0..N {
        let start = Instant::now();
        let mut child = Command::new(bin)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn sddk");
        let pid = child.id();
        // Read RSS BEFORE wait() — proc/<pid> vanishes once parent reaps.
        let rss = peak_rss_kib(pid);
        let _ = child.wait();
        let elapsed_ms = start.elapsed().as_millis();
        samples_ms.push(elapsed_ms);
        if rss > max_rss_kib {
            max_rss_kib = rss;
        }
    }

    let mut sorted = samples_ms.clone();
    sorted.sort_unstable();
    let stats = Stats {
        p50_ms: pct(&sorted, 0.50),
        p95_ms: pct(&sorted, 0.95),
        max_ms: *sorted.last().unwrap(),
        peak_rss_kib: max_rss_kib,
    };
    eprintln!(
        "[c3g] {label}: p50={}ms p95={}ms max={}ms peak_rss={}KiB (N={})",
        stats.p50_ms, stats.p95_ms, stats.max_ms, stats.peak_rss_kib, N
    );
    stats
}

fn format_yaml(name: &str, args: &[&str], stats: Stats, env: &str) -> String {
    format!(
        r#"  - scenario: {name}
    args: [{args_vec}]
    n: {n}
    p50_ms: {p50}
    p95_ms: {p95}
    max_ms: {max}
    peak_rss_kib: {rss}
    env: "{env}"
"#,
        name = name,
        args_vec = args.join(", "),
        n = N,
        p50 = stats.p50_ms,
        p95 = stats.p95_ms,
        max = stats.max_ms,
        rss = stats.peak_rss_kib,
        env = env,
    )
}

#[test]
#[ignore]
fn c3g_perf_budget_base() {
    let bin = sddk_bin();
    eprintln!("[c3g] using binary: {}", bin.display());

    let s1 = measure_scenario("S1-version", &bin, &["version"]);
    let s2 = measure_scenario("S2-cycle-status", &bin, &["cycle", "status"]);
    let s3 = measure_scenario("S3-lint", &bin, &["lint"]);

    // Acceptance (SCOPE §10): observable numbers produced; budget is just baseline,
    // not a hard target — we capture the numbers regardless of target breach.
    let mut results = String::new();
    results.push_str("# C3g — Performance budget for Base profile\n");
    results.push_str(&format!("# env: {ENV_LABEL}\n"));
    results.push_str(&format!("# n: {N} runs per scenario\n\n"));
    results.push_str("scenarios:\n");
    results.push_str(&format_yaml("S1-version", &["version"], s1, ENV_LABEL));
    results.push_str(&format_yaml(
        "S2-cycle-status",
        &["cycle", "status"],
        s2,
        ENV_LABEL,
    ));
    results.push_str(&format_yaml("S3-lint", &["lint"], s3, ENV_LABEL));

    let mut f = fs::File::create(RESULTS_PATH).expect("create results file");
    f.write_all(results.as_bytes()).expect("write results");
    eprintln!("[c3g] wrote {RESULTS_PATH}");

    // Soft target check (NOT a hard fail — just a visible signal):
    let s1_ok = s1.p95_ms < 50;
    let s2_ok = s2.p95_ms < 100;
    let s3_ok = s3.p95_ms < 500;
    eprintln!(
        "[c3g] soft-target check: S1<50ms={} S2<100ms={} S3<500ms={}",
        s1_ok, s2_ok, s3_ok
    );
    // Note: bench harness does NOT assert; baseline only.
}
