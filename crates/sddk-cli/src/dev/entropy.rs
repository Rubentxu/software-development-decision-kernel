//! `dev entropy` — multidimensional architecture health report.
//!
//! Produces an advisory report covering:
//! - LOC per crate and per module
//! - Fan-in / fan-out per crate
//! - Coupling pairs (cross-crate import edges)
//! - Large files (LOC threshold)
//! - Test-to-code ratio per crate
//!
//! Advisory only: exit code is always 0. Use `--strict` to exit 1 on any WARN.

use crate::{CliEnvironment, CommandOutput};
use sddk_engine::rules::BaselineConsumer;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Threshold for "large file" warning (inclusive).
const LARGE_FILE_LOC_THRESHOLD: usize = 500;

/// Threshold for high fan-out (inclusive).
const HIGH_FAN_OUT: usize = 5;

/// Threshold for high fan-in (inclusive).
const HIGH_FAN_IN: usize = 10;

/// Output format for the entropy report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EntropyReport {
    pub schema_version: &'static str,
    pub root: String,
    pub crates: Vec<CrateMetrics>,
    pub coupling_pairs: Vec<CouplingPair>,
    pub large_files: Vec<FileMetric>,
    pub summary: ReportSummary,
}

/// Per-crate metrics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CrateMetrics {
    pub name: String,
    pub loc: usize,
    pub files: usize,
    pub test_files: usize,
    pub fan_in: usize,  // crates that import this crate
    pub fan_out: usize, // crates this crate imports
    pub status: MetricStatus,
}

/// A directed coupling edge between two crates.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CouplingPair {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub count: usize,
}

/// A file that exceeds the LOC threshold.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FileMetric {
    pub path: String,
    pub crate_name: String,
    pub loc: usize,
    pub status: MetricStatus,
}

/// High-level summary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReportSummary {
    pub total_loc: usize,
    pub total_crates: usize,
    pub total_large_files: usize,
    pub total_coupling_edges: usize,
    pub entropy_score: f64, // 0.0 = perfect, 1.0 = chaotic
}

/// Simple status for each metric.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricStatus {
    Pass,
    Warn,
    Info,
}

impl MetricStatus {
    #[allow(dead_code)]
    fn from_bool(warn: bool) -> Self {
        if warn {
            MetricStatus::Warn
        } else {
            MetricStatus::Pass
        }
    }
}

/// Collects all `.rs` source files under `crates/<crate>/src`.
fn collect_src_files(root: &Path) -> HashMap<String, Vec<(String, usize)>> {
    let mut by_crate: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    let crates_dir = root.join("crates");

    let Ok(entries) = std::fs::read_dir(&crates_dir) else {
        return by_crate;
    };

    for entry in entries.flatten() {
        let crate_name = entry.file_name().to_string_lossy().into_owned();
        let src_dir = entry.path().join("src");

        let entries = walkdir::WalkDir::new(&src_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok());

        let mut files = Vec::new();
        for e in entries {
            if e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "rs") {
                let path_str = e.path().display().to_string();
                let loc = count_lines(e.path()).unwrap_or(0);
                files.push((path_str, loc));
            }
        }

        if !files.is_empty() {
            by_crate.insert(crate_name, files);
        }
    }

    by_crate
}

/// Counts non-empty lines in a Rust source file.
fn count_lines(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Returns true if a file path looks like a test file.
fn is_test_file(path: &str) -> bool {
    path.contains("/tests/") || path.ends_with("_tests.rs") || path.ends_with("_test.rs")
}

/// Builds the entropy report from the live workspace.
pub fn build_report(root: &Path) -> Result<EntropyReport, String> {
    let root_str = root.display().to_string();

    // ── LOC metrics ──────────────────────────────────────────────────────────
    let by_crate = collect_src_files(root);

    // ── Fan-in / fan-out from live baseline ────────────────────────────────────
    let baseline = BaselineConsumer::capture_live(root)
        .map_err(|e| format!("baseline capture failed: {e}"))?;

    // fan_out[crate] = set of crates it imports
    let mut fan_out: HashMap<String, HashSet<String>> = HashMap::new();
    // fan_in[crate] = set of crates that import it
    let mut fan_in: HashMap<String, HashSet<String>> = HashMap::new();

    for edge in &baseline.cross_crate_imports {
        fan_out
            .entry(edge.from_crate.clone())
            .or_default()
            .insert(edge.to_crate.clone());
        fan_in
            .entry(edge.to_crate.clone())
            .or_default()
            .insert(edge.from_crate.clone());
    }

    // All crates seen (union of LOC crates and baseline crates)
    let mut all_crates: HashSet<String> = by_crate.keys().cloned().collect();
    for k in fan_out.keys() {
        all_crates.insert(k.clone());
    }
    for k in fan_in.keys() {
        all_crates.insert(k.clone());
    }

    let mut crates_metrics: Vec<CrateMetrics> = by_crate
        .iter()
        .map(|(name, files)| {
            let loc: usize = files.iter().map(|(_, l)| l).sum();
            let test_files = files.iter().filter(|(p, _)| is_test_file(p)).count();
            let fan_in_count = fan_in.get(name).map(|s| s.len()).unwrap_or(0);
            let fan_out_count = fan_out.get(name).map(|s| s.len()).unwrap_or(0);
            let status = if fan_out_count > HIGH_FAN_OUT || fan_in_count > HIGH_FAN_IN {
                MetricStatus::Warn
            } else {
                MetricStatus::Pass
            };
            CrateMetrics {
                name: name.clone(),
                loc,
                files: files.len(),
                test_files,
                fan_in: fan_in_count,
                fan_out: fan_out_count,
                status,
            }
        })
        .collect();

    // Also include crates with no source files but fan-in/out from baseline
    for name in all_crates {
        if by_crate.contains_key(&name) {
            continue;
        }
        let fan_in_count = fan_in.get(&name).map(|s| s.len()).unwrap_or(0);
        let fan_out_count = fan_out.get(&name).map(|s| s.len()).unwrap_or(0);
        crates_metrics.push(CrateMetrics {
            name,
            loc: 0,
            files: 0,
            test_files: 0,
            fan_in: fan_in_count,
            fan_out: fan_out_count,
            status: MetricStatus::Info,
        });
    }

    crates_metrics.sort_by_key(|c| c.name.clone());

    // ── Large files ───────────────────────────────────────────────────────────
    let mut large_files: Vec<FileMetric> = Vec::new();
    for (crate_name, files) in &by_crate {
        for (path, loc) in files {
            if *loc >= LARGE_FILE_LOC_THRESHOLD {
                large_files.push(FileMetric {
                    path: path.clone(),
                    crate_name: crate_name.clone(),
                    loc: *loc,
                    status: MetricStatus::Warn,
                });
            }
        }
    }
    large_files.sort_by_key(|f| f.loc);
    large_files.reverse();

    // ── Coupling pairs ────────────────────────────────────────────────────────
    let mut coupling_map: HashMap<(String, String), usize> = HashMap::new();
    for edge in &baseline.cross_crate_imports {
        *coupling_map
            .entry((edge.from_crate.clone(), edge.to_crate.clone()))
            .or_default() += 1;
    }
    let coupling_pairs: Vec<CouplingPair> = coupling_map
        .into_iter()
        .map(|((from, to), count)| CouplingPair {
            from,
            to,
            kind: "use".to_string(),
            count,
        })
        .collect();

    // ── Summary ──────────────────────────────────────────────────────────────
    let total_loc: usize = crates_metrics.iter().map(|c| c.loc).sum();
    let total_crates = crates_metrics.len();
    let total_large_files = large_files.len();
    let total_coupling_edges = coupling_pairs.len();

    // Heuristic entropy score: penalize coupling edges and large files
    let coupling_component = if total_loc > 0 {
        (total_coupling_edges as f64) / (total_loc as f64 / 1000.0).max(1.0)
    } else {
        0.0
    };
    let file_component = (total_large_files as f64) / (total_crates as f64).max(1.0);
    let entropy_score = (coupling_component + file_component * 2.0).min(1.0);

    let summary = ReportSummary {
        total_loc,
        total_crates,
        total_large_files,
        total_coupling_edges,
        entropy_score: (entropy_score * 100.0).round() / 100.0,
    };

    Ok(EntropyReport {
        schema_version: "1.0.0",
        root: root_str,
        crates: crates_metrics,
        coupling_pairs,
        large_files,
        summary,
    })
}

/// Render the report as human-readable text.
pub fn render_text(report: &EntropyReport) -> String {
    let mut out = String::new();

    // Header
    out.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    out.push_str("║               SDDK Architecture Entropy Report              ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

    // Summary
    out.push_str("┌─ Summary ────────────────────────────────────────────────────┐\n");
    out.push_str(&format!(
        "│  Total LOC        {:>8}                               │\n",
        report.summary.total_loc
    ));
    out.push_str(&format!(
        "│  Crates          {:>8}                               │\n",
        report.summary.total_crates
    ));
    out.push_str(&format!(
        "│  Coupling edges  {:>8}                               │\n",
        report.summary.total_coupling_edges
    ));
    out.push_str(&format!(
        "│  Large files     {:>8}                               │\n",
        report.summary.total_large_files
    ));
    out.push_str(&format!(
        "│  Entropy score   {:>8.2} (0=perfect, 1=chaotic)      │\n",
        report.summary.entropy_score
    ));
    out.push_str("└────────────────────────────────────────────────────────────┘\n\n");

    // Per-crate table
    out.push_str("┌─ Crate Metrics ────────────────────────────────────────────┐\n");
    out.push_str(&format!(
        "│ {:20} {:>7} {:>6} {:>5} {:>8} {:>8} │\n",
        "CRATE", "LOC", "FILES", "TESTS", "FAN-IN", "FAN-OUT"
    ));
    out.push_str("│----------------------- ------- ------ ----- -------- -------- │\n");

    for c in &report.crates {
        let status_ch = match c.status {
            MetricStatus::Warn => '⚠',
            MetricStatus::Info => '·',
            MetricStatus::Pass => ' ',
        };
        out.push_str(&format!(
            "│ {} {:19} {:>7} {:>6} {:>5} {:>8} {:>8} │\n",
            status_ch, c.name, c.loc, c.files, c.test_files, c.fan_in, c.fan_out
        ));
    }
    out.push_str("└────────────────────────────────────────────────────────────┘\n\n");

    // Large files
    if !report.large_files.is_empty() {
        out.push_str(&format!(
            "┌─ Large Files (≥{} LOC) ───────────────────────────────────┐\n",
            LARGE_FILE_LOC_THRESHOLD
        ));
        for f in &report.large_files {
            out.push_str(&format!(
                "│  {:>6} LOC  {:45} │\n",
                f.loc,
                abbreviate_path(&f.path)
            ));
        }
        out.push_str("└────────────────────────────────────────────────────────────┘\n\n");
    }

    // Top coupling pairs
    if !report.coupling_pairs.is_empty() {
        out.push_str("┌─ Top Coupling Edges ─────────────────────────────────────┐\n");
        let mut pairs = report.coupling_pairs.clone();
        pairs.sort_by_key(|p| p.count);
        pairs.reverse();
        for p in pairs.iter().take(10) {
            out.push_str(&format!(
                "│  {:20} → {:20}  ({:>3} edges)    │\n",
                p.from, p.to, p.count
            ));
        }
        if pairs.len() > 10 {
            out.push_str(&format!(
                "│  … and {} more edges                                         │\n",
                pairs.len() - 10
            ));
        }
        out.push_str("└────────────────────────────────────────────────────────────┘\n");
    }

    out
}

/// Abbreviate a path to fit in a column.
fn abbreviate_path(path: &str) -> String {
    if path.len() <= 45 {
        return path.to_string();
    }
    let start = path.len() - 42;
    format!("…{}", &path[start..])
}

pub fn run_dev_entropy(args: super::EntropyArgs, _environment: &CliEnvironment) -> CommandOutput {
    let root = args.root.as_path();
    let report = match build_report(root) {
        Ok(r) => r,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error: {e}\n"),
            };
        }
    };

    let status = if args.strict && report.summary.total_large_files > 0 {
        1
    } else {
        0
    };

    let output = match args.format {
        super::EntropyFormat::Text => render_text(&report),
        super::EntropyFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(s) => s,
            Err(e) => {
                return CommandOutput {
                    status: 1,
                    stdout: String::new(),
                    stderr: format!("error: failed to serialize JSON: {e}\n"),
                };
            }
        },
    };

    CommandOutput {
        status,
        stdout: output,
        stderr: String::new(),
    }
}
