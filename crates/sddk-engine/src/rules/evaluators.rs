//! Real evaluators for ARCH001..015.

// `missing_docs` is allowed across this file because the Phase 1 ARCH
// evaluators were introduced before the workspace-wide
// `#![warn(missing_docs)]` activation. A future docs-pass cycle should
// restore the per-item `///` doc comments and remove this allow.
#![allow(missing_docs)]

use regex::RegexSet;
use sddk_domain::{EvaluatorKind, RuleEvaluation, RuleRegistry, RuleStatus};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::Baseline;

pub const EVALUATOR_VERSION: &str = "0.1.0";

/// Evaluates every registered rule against the baseline (Phase 1).
///
/// Waiver precedence: if a waiver exists and is still active for the
/// baseline's `head_anchor` (decided by the injected
/// [`WaiverExpiryResolver`], INC-DEBT-018), the evaluation is overridden to
/// `Waived`. Expired waivers result in `NotApplicable` to preserve Phase 0
/// backward compatibility with existing waivers in the registry.
/// Builds the production waiver-expiry resolver: real git ancestry.
///
/// A waiver is active when `granted_until_sha` is an ancestor of (or equal
/// to) `head_anchor` in the repository at `repo_root`, determined via
/// `git merge-base --is-ancestor`. Falls back to lexicographic comparison
/// when the resolved SHA is unknown to git (`unknown`, short-vs-long SHA
/// resolution failure) so evaluation never silently waives on bad data.
/// See INC-DEBT-018.
#[must_use]
pub fn git_ancestry_resolver(repo_root: &std::path::Path) -> sddk_domain::WaiverExpiryResolver {
    let root = repo_root.to_path_buf();
    std::sync::Arc::new(move |head: &str, until: &str| {
        if head == until {
            return true;
        }
        // Resolve both anchors to full SHAs; if either is not a git object
        // (e.g. "unknown"), fall back to lexicographic compare.
        let (full_head, full_until) =
            match (git_rev_parse(&root, head), git_rev_parse(&root, until)) {
                (Some(h), Some(u)) => (h, u),
                _ => return head <= until,
            };
        if full_head == full_until {
            return true;
        }
        git_is_ancestor(&root, &full_until, &full_head)
    })
}

fn git_rev_parse(repo_root: &std::path::Path, rev: &str) -> Option<String> {
    let out = std::process::Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        None
    }
}

fn git_is_ancestor(repo_root: &std::path::Path, ancestor: &str, descendant: &str) -> bool {
    std::process::Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `root` is the checkout the tree-reading rules (ARCH004, ARCH005) measure
/// against. `None` means "no tree was supplied", and those two rules then
/// report `NotApplicable` with a provenance that says the measurement was NOT
/// performed — never a `Pass` they did not earn.
pub fn evaluate_all(
    registry: &RuleRegistry,
    baseline: &Baseline,
    evaluated_at: &str,
    root: Option<&Path>,
) -> Vec<RuleEvaluation> {
    // Legacy lexicographic resolver (head_anchor <= granted_until_sha).
    let resolver: sddk_domain::WaiverExpiryResolver =
        std::sync::Arc::new(|head: &str, until: &str| head <= until);
    evaluate_all_with_resolver(registry, baseline, evaluated_at, resolver, root)
}

/// Like [`evaluate_all`], but the waiver-expiry decision comes from the
/// injected resolver. Production callers inject git-ancestry semantics
/// (`merge-base --is-ancestor`); tests and no-git environments fall back to
/// the lexicographic comparison of [`evaluate_all`].
pub fn evaluate_all_with_resolver(
    registry: &RuleRegistry,
    baseline: &Baseline,
    evaluated_at: &str,
    resolver: sddk_domain::WaiverExpiryResolver,
    root: Option<&Path>,
) -> Vec<RuleEvaluation> {
    registry
        .iter()
        .map(|rule| {
            // ── Waiver pre-check ──────────────────────────────────────────────
            if let Some(w) = registry.waiver_for(&rule.id) {
                let head = baseline.ref_.head_anchor.as_str();
                let until = w.granted_until_sha.as_str();
                // Sentinel "9999…" never expires regardless of resolver semantics.
                let active =
                    until == sddk_domain::WAIVER_NO_EXPIRY_SENTINEL || resolver(head, until);
                if active {
                    return RuleEvaluation {
                        rule_id: rule.id.clone(),
                        status: RuleStatus::Waived,
                        observed: json!({
                            "waiver_id": w.id,
                            "reason": w.reason
                        }),
                        baseline_sha256: baseline.ref_.sha256.clone(),
                        evaluated_at: evaluated_at.to_owned(),
                        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                        waiver_id: Some(w.id.clone()),
                        evaluator_kind: EvaluatorKind::Schema,
                        evaluator_version: EVALUATOR_VERSION.to_owned(),
                        provenance: None,
                    };
                }
                // Waiver expired → NotApplicable (Phase 0 backward compat)
                return RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({ "phase": "phase0", "rule_id": rule.id }),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!(
                        "waiver {} expired at baseline {}",
                        w.id, baseline.ref_.head_anchor
                    )),
                };
            }

            // ── Rule-specific evaluation ─────────────────────────────────────
            match rule.id.as_str() {
                "ARCH001" => evaluate_arch001(rule, baseline, evaluated_at),
                "ARCH002" => evaluate_arch002(rule, baseline, evaluated_at),
                "ARCH003" => evaluate_arch003(rule, baseline, evaluated_at),
                "ARCH004" => evaluate_arch004(rule, baseline, evaluated_at, root),
                "ARCH005" => evaluate_arch005(rule, baseline, evaluated_at, root),
                "ARCH006" => evaluate_arch006(rule, baseline, evaluated_at),
                "ARCH007" => evaluate_arch007(rule, baseline, evaluated_at),
                "ARCH008" => evaluate_arch008(rule, baseline, evaluated_at),
                "ARCH009" => evaluate_arch009(rule, baseline, evaluated_at),
                "ARCH010" => evaluate_arch010(rule, baseline, evaluated_at),
                "ARCH011" => evaluate_arch011(rule, baseline, evaluated_at),
                "ARCH012" => evaluate_arch012(rule, baseline, evaluated_at),
                "ARCH013" => evaluate_arch013(rule, baseline, evaluated_at),
                "ARCH014" => evaluate_arch014(rule, baseline, evaluated_at),
                "ARCH015" => evaluate_arch015(rule, baseline, evaluated_at),
                _ => RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({}),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!("evaluator not implemented for {}", rule.id)),
                },
            }
        })
        .collect()
}

// ── ARCH001 ──────────────────────────────────────────────────────────────────

/// engine_must_not_depend_on_storage: Fail if any edge from sddk-engine to sddk-storage
/// exists in the baseline (Cargo dep or use statement).
fn evaluate_arch001(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-engine" && e.to_crate == "sddk-storage")
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH001 live evaluator: checks sddk-engine→sddk-storage edges in \
             cross_crate_imports (Cargo deps + use statements)"
                .to_owned(),
        ),
    }
}

// ── ARCH002 ──────────────────────────────────────────────────────────────────

/// domain_must_not_depend_on_adapters: Fail if any edge from sddk-domain to
/// sddk-storage, sddk-gateway, or sddk-cli exists.
fn evaluate_arch002(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let forbidden = ["sddk-storage", "sddk-gateway", "sddk-cli"];
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-domain" && forbidden.contains(&e.to_crate.as_str()))
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH002 live evaluator: checks sddk-domain→{storage,gateway,cli} edges".to_owned(),
        ),
    }
}

// ── ARCH003 ──────────────────────────────────────────────────────────────────

/// production_crates_must_not_depend_on_storage_directly: Fail if any production
/// crate has a source-level `use` edge to `sddk-storage`, unless the edge is
/// from `sddk-storage` itself (internal use is fine) or the crate provides
/// `LedgerFactory` (P1-FIX-005).
///
/// This is the extended form of the original "cli must not own persistence logic"
/// rule: it covers ALL crates, not just CLI.  The exception for `LedgerFactory`
/// providers allows a crate to use `sddk-storage` when it has been explicitly
/// composed through the factory port (P1-FIX-002 / ADR-0021).
fn evaluate_arch003(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    use crate::rules::baseline::CrossCrateImportKind;

    // Known crates that provide LedgerFactory (implement or re-export the trait).
    // These may import from sddk-storage without triggering a violation.
    const LEDGER_FACTORY_PROVIDERS: &[&str] = &["sddk-domain", "sddk-storage"];

    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| {
            // Only source-level edges
            if e.kind != CrossCrateImportKind::Use {
                return false;
            }
            // Must be an edge to sddk-storage
            if e.to_crate != "sddk-storage" {
                return false;
            }
            // sddk-storage using itself is always fine
            if e.from_crate == "sddk-storage" {
                return false;
            }
            // Crates that provide LedgerFactory are allowed
            if LEDGER_FACTORY_PROVIDERS.contains(&e.from_crate.as_str()) {
                return false;
            }
            true
        })
        .map(|e| {
            json!({
                "from_crate": e.from_crate,
                "from_file": e.from_file,
                "line": e.line,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH003 live evaluator: any production crate using sddk-storage directly \
             is a violation unless the crate provides LedgerFactory (P1-FIX-005)"
                .to_owned(),
        ),
    }
}

// ── ARCH004 ──────────────────────────────────────────────────────────────────

/// Veredicto honesto cuando no hay árbol contra el que medir.
///
/// Estas dos leyes (ARCH004, ARCH005) son las únicas que leen el disco, y un
/// `NotApplicable` que no distingue "no había nada que medir" de "no medí"
/// es exactamente el defecto que este ciclo viene a cerrar: el evaluador
/// anterior decía `NotApplicable` con un motivo falso (`kernel repo, not a
/// pack host`) sobre un repo que SÍ tiene pack. Por eso el motivo dice
/// exactamente qué falta, y por eso el estado NO es `Pass`.
fn not_measured(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
    what: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({ "measured": false }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(format!(
            "NOT MEASURED: no repository root was supplied, so {what} was never looked at. \
             This is an absent measurement, not a clean one."
        )),
    }
}

/// Filename locator for pack manifests: `packs/<name>/manifest.toml`.
fn pack_manifests(root: &Path) -> Vec<PathBuf> {
    let packs = root.join("packs");
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&packs) {
        for entry in entries.flatten() {
            let manifest = entry.path().join("manifest.toml");
            if manifest.is_file() {
                out.push(manifest);
            }
        }
    }
    out.sort();
    out
}

/// Reads `[dependencies]` (requires / integrates_with / conflicts_with) from a
/// pack manifest. Returns `Err` with the reason when the TOML does not parse or
/// the table is malformed — an unreadable manifest must not read as "declared
/// nothing, therefore clean".
fn declared_pack_deps(manifest: &Path) -> Result<Vec<(String, String)>, String> {
    let raw = fs::read_to_string(manifest).map_err(|e| e.to_string())?;
    let value: toml::Value = toml::from_str(&raw).map_err(|e| e.to_string())?;
    let deps = value.get("dependencies").ok_or("no [dependencies] table")?;
    let table = deps.as_table().ok_or("[dependencies] is not a table")?;
    let mut out = Vec::new();
    for (field, entry) in table {
        let list = entry
            .as_array()
            .ok_or_else(|| format!("dependencies.{field} is not an array"))?;
        for item in list {
            let name = item
                .as_str()
                .ok_or_else(|| format!("dependencies.{field} holds a non-string"))?;
            out.push((field.clone(), name.to_owned()));
        }
    }
    Ok(out)
}

/// The set of crate names that actually exist: every `[package] name` under
/// `crates/*/Cargo.toml`, plus the workspace `members`.
fn real_crate_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join("crates")) {
        for entry in entries.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            let Ok(raw) = fs::read_to_string(&manifest) else {
                continue;
            };
            let Ok(value) = raw.parse::<toml::Value>() else {
                continue;
            };
            if let Some(name) = value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    names
}

/// The `sddk-*` dependencies a pack crate really has, from its Cargo.toml.
///
/// `path` and `workspace = true` both count: a workspace-inherited
/// `sddk-domain` is still a dependency the manifest must account for.
fn actual_crate_deps(root: &Path, pack_name: &str) -> Vec<String> {
    let manifest = root.join("crates").join(pack_name).join("Cargo.toml");
    let Ok(raw) = fs::read_to_string(&manifest) else {
        return Vec::new();
    };
    let Ok(value) = raw.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = value.get(key).and_then(|d| d.as_table()) else {
            continue;
        };
        for (name, spec) in table {
            let is_sddk = name.starts_with("sddk-");
            let points_at_path = spec.get("path").is_some();
            let inherits = spec
                .get("workspace")
                .and_then(|w| w.as_bool())
                .unwrap_or(false);
            if is_sddk && (points_at_path || inherits) {
                out.push(name.clone());
            }
        }
    }
    out.sort();
    out
}

/// `packs_must_declare_dependencies`: the pack's manifest must account for
/// every dependency the pack actually has, and must not name one that isn't
/// there.
///
/// Both directions, because the law is about the manifest *matching* reality,
/// and each direction catches a different lie:
///   - undeclared: `crates/sddk-pack-uat` depends on `sddk-domain`, and the
///     manifest's `requires` never says so — the pack under-declares.
///   - dangling: the manifest names `sddk-core`, which no crate provides — the
///     pack over-declares a relationship with something absent.
///
/// The previous evaluator answered `NotApplicable` with "kernel repo, not a
/// pack host". This repo ships a pack, so that reason was false and the rule
/// was silent about a real mismatch.
fn evaluate_arch004(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
    root: Option<&Path>,
) -> RuleEvaluation {
    let Some(root) = root else {
        return not_measured(rule, baseline, evaluated_at, "the pack manifests");
    };

    let real = real_crate_names(root);
    let manifests = pack_manifests(root);
    let mut violations = Vec::new();
    let mut packs = Vec::new();

    for manifest in &manifests {
        let rel = manifest
            .strip_prefix(root)
            .unwrap_or(manifest)
            .to_string_lossy()
            .into_owned();
        let pack_dir = manifest
            .parent()
            .and_then(Path::file_name)
            .unwrap_or_default();
        let pack_name = pack_dir.to_string_lossy().into_owned();

        let declared = match declared_pack_deps(manifest) {
            Ok(d) => d,
            Err(e) => {
                // An unreadable manifest is a violation, not a pass: "could not
                // read" and "declared nothing" must not share a verdict.
                violations.push(json!({
                    "pack": pack_name,
                    "manifest": rel,
                    "kind": "unreadable_manifest",
                    "detail": e,
                }));
                continue;
            }
        };

        let mut declared_names: Vec<&str> = declared.iter().map(|(_, n)| n.as_str()).collect();

        // Direction 1 — dangling: names with no crate behind them.
        for (field, name) in &declared {
            if !real.iter().any(|r| r == name) {
                violations.push(json!({
                    "pack": pack_name,
                    "manifest": rel,
                    "kind": "dangling_declaration",
                    "field": field,
                    "declared": name,
                    "detail": format!("{name} is declared in {field} but no crate provides it"),
                }));
            }
        }

        // Direction 2 — undeclared: real dependencies the manifest omits.
        let actual = actual_crate_deps(root, &pack_name);
        for dep in &actual {
            if !declared_names.contains(&dep.as_str()) {
                violations.push(json!({
                    "pack": pack_name,
                    "manifest": rel,
                    "kind": "undeclared_dependency",
                    "actual": dep,
                    "detail": format!(
                        "crates/{pack_name} depends on {dep}, which [dependencies] never names"
                    ),
                }));
            }
        }
        declared_names.sort_unstable();

        packs.push(json!({
            "manifest": rel,
            "declared": declared_names,
            "actual": actual,
        }));
    }

    let status = if violations.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    let dangling = violations
        .iter()
        .filter(|v| v.get("kind").and_then(|k| k.as_str()) == Some("dangling_declaration"))
        .count();
    let undeclared = violations
        .iter()
        .filter(|v| v.get("kind").and_then(|k| k.as_str()) == Some("undeclared_dependency"))
        .count();
    let unreadable = violations.len() - dangling - undeclared;

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "packs": packs,
            "violations": violations,
            "count": violations.len(),
            "real_crates": real.len(),
            "summary": if violations.is_empty() {
                format!(
                    "{} pack manifest(s) match the {} real crate(s)",
                    packs.len(),
                    real.len()
                )
            } else {
                format!(
                    "{dangling} dangling declaration(s), {undeclared} undeclared dependency(ies), {unreadable} unreadable manifest(s)"
                )
            },
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(format!(
            "live evaluator: {} pack manifest(s) checked against {} real crate(s), in both \
             directions (dangling declarations and undeclared actual dependencies)",
            packs.len(),
            real.len()
        )),
    }
}

// ── ARCH005 ──────────────────────────────────────────────────────────────────

/// Governed effects, in the vocabulary of SPEC-009: anything that writes to a
/// durable location or leaves the machine. `Pure` and `Read` are NOT governed,
/// which is why they are absent here — a reactive verifier that reads the tree
/// is conforming.
///
/// Each entry is (label, RegexSet pattern). These are the shapes that perform
/// an effect without asking the capability layer first.
static GOVERNED_EFFECT_PATTERNS: &[(&str, &str)] = &[
    (
        "durable_write",
        r"\bfs::(write|remove_file|remove_dir_all|rename)\b",
    ),
    ("file_create", r"\bFile::create\b"),
    ("subprocess", r"\b(Command::new|process::Command)\b"),
    (
        "network_egress",
        r"\b(reqwest::|ureq::|TcpStream::connect)\b",
    ),
    (
        "storage_adapter",
        r"\b(sddk_storage|SqliteXStore|SqliteControlPlane)\b",
    ),
    ("gateway_egress", r"\bsddk_gateway\b"),
];

/// Locates the reactive-behavior modules: any `.rs` file under `crates/` whose
/// file stem or parent directory is `reactive`.
///
/// A locator, not a hand-maintained list. The failure mode of a hardcoded list
/// is that a new reactive module is invisible and the rule silently stops
/// covering it; the failure mode of a name locator is a module named something
/// else, which the provenance below makes checkable by a reader.
fn reactive_behavior_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let is_rust = path.extension().and_then(|e| e.to_str()) == Some("rs");
                if is_rust && (stem == "reactive" || stem.starts_with("reactive_")) {
                    out.push(path);
                }
            } else if path.is_dir() && path.file_name().and_then(|n| n.to_str()) == Some("reactive")
            {
                collect_rust(&path, out);
            } else if path.is_dir() {
                walk(&path, out);
            }
        }
    }
    fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rust(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    walk(&root.join("crates"), &mut out);
    out.sort();
    out
}

/// `reactive_behaviors_must_not_execute_governed_effects_directly`: a reactive
/// behavior decides and verifies; it must not itself write, spawn, dial out or
/// reach an adapter, because those are the effects the capability layer gates.
///
/// The previous evaluator answered `NotApplicable` with "Phase 5 reactive
/// runtime not yet shipped". `crates/sddk-engine/src/reactive_verify.rs` is 394
/// lines and `run_reactive_verify` is its entry point, so the reason was false:
/// the runtime shipped, it conforms, and the rule had never checked.
fn evaluate_arch005(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
    root: Option<&Path>,
) -> RuleEvaluation {
    let Some(root) = root else {
        return not_measured(
            rule,
            baseline,
            evaluated_at,
            "the reactive behavior modules",
        );
    };

    let files = reactive_behavior_files(root);
    let mut violations = Vec::new();
    let mut subjects = Vec::new();

    for file in &files {
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .into_owned();
        let Ok(content) = fs::read_to_string(file) else {
            violations.push(json!({
                "file": rel,
                "effect": "unreadable_file",
                "line": 0,
                "text": "",
            }));
            continue;
        };
        let mut hits = 0usize;
        for (line_no, line) in content.lines().enumerate() {
            // Comments and doc comments state the rule; they are not the rule
            // being broken. Matching them would make the module's own
            // documentation of the constraint a violation of it.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            for (label, pattern) in GOVERNED_EFFECT_PATTERNS {
                if regex::Regex::new(pattern)
                    .expect("static pattern")
                    .is_match(line)
                {
                    violations.push(json!({
                        "file": rel,
                        "effect": label,
                        "line": (line_no + 1) as u32,
                        "text": line.trim(),
                    }));
                    hits += 1;
                }
            }
        }
        subjects.push(json!({
            "file": rel,
            "lines": content.lines().count(),
            "governed_effect_hits": hits,
        }));
    }

    // No subject means the law has nothing to say — but that must be a
    // statement about the tree, never a silent Pass.
    if files.is_empty() {
        return RuleEvaluation {
            rule_id: rule.id.clone(),
            status: RuleStatus::NotApplicable,
            observed: json!({ "subjects": [], "count": 0, "measured": true }),
            baseline_sha256: baseline.ref_.sha256.clone(),
            evaluated_at: evaluated_at.to_owned(),
            evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
            waiver_id: None,
            evaluator_kind: EvaluatorKind::Heuristic,
            evaluator_version: EVALUATOR_VERSION.to_owned(),
            provenance: Some(
                "MEASURED: no reactive behavior module exists under crates/ \
                 (locator: .rs files whose stem is `reactive` or `reactive_*`). \
                 The law has no subject here; this is not a conformance claim."
                    .to_owned(),
            ),
        };
    }

    let status = if violations.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "subjects": subjects,
            "violations": violations,
            "count": violations.len(),
            "vocabulary": GOVERNED_EFFECT_PATTERNS
                .iter()
                .map(|(l, _)| *l)
                .collect::<Vec<_>>(),
            "summary": format!(
                "{} governed-effect hit(s) across {} reactive module(s)",
                violations.len(),
                files.len()
            ),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(format!(
            "live evaluator: {} reactive behavior module(s) scanned for {} governed-effect \
             shape(s). Reading the tree is `Read`, not governed; writing, spawning, dialing and \
             adapter reach are.",
            files.len(),
            GOVERNED_EFFECT_PATTERNS.len()
        )),
    }
}

// ── Evaluadores de arista (C5) ───────────────────────────────────────────────
//
// Las seis reglas siguientes declaraban una arista prohibida y no tenian
// evaluador: caian en el `_ =>` del dispatcher, que devuelve `NotApplicable`
// con `provenance: "evaluator not implemented"`. Un veredicto que dice
// "no aplica" no es "no hay", asi que el gate podia declarar conformidad sobre
// seis leyes que no habia mirado nunca.
//
// Las seis son LA MISMA medicion: una arista `from_crate -> to_crate` sobre
// `baseline.cross_crate_imports`. Copiar el bloque de ARCH001 seis veces
// serian seis copias que divergen en cuanto una cambia, luego la forma esta
// factorizada en `evaluate_forbidden_edge` y cada regla la llama con sus dos
// argumentos y nada mas.
//
// El septimo argumento, `file_scope`, distingue la unica que no es una arista
// entre crates sino la arista de UN modulo: ARCH006 se declara sobre el
// modulo `graph` de `sddk-domain`, no sobre el crate entero.

/// Shared edge matcher: fail when any `from -> to` edge survives `file_scope`.
///
/// `file_scope`, when `Some`, narrows the measurement to one exact file. It is
/// what keeps ARCH006 measuring the graph projection instead of the whole
/// crate. Exact path, not prefix: with a prefix, `crates/sddk-domain/src/graph`
/// measures what the rule declares today only because no other file starts
/// with `graph` yet, and the day `graph_builder.rs` appears the rule would
/// measure more than it declares with nothing noticing. A scope that depends
/// on the future alphabet of names is not a scope.
fn evaluate_forbidden_edge(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
    from_crate: &str,
    forbidden: &[&str],
    file_scope: Option<&str>,
) -> RuleEvaluation {
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == from_crate && forbidden.contains(&e.to_crate.as_str()))
        .filter(|e| match file_scope {
            Some(path) => e.from_file == path,
            None => true,
        })
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
            "scope": file_scope,
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(format!(
            "live evaluator: {from_crate}->{{{}}} edges in cross_crate_imports{}",
            forbidden.join(","),
            match file_scope {
                Some(path) => format!(" scoped to {path}"),
                None => String::new(),
            }
        )),
    }
}

/// ARCH006: the `sddk-domain` graph projection must depend only on inward
/// ports, never on the engine.
fn evaluate_arch006(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-domain",
        &["sddk-engine"],
        Some("crates/sddk-domain/src/graph.rs"),
    )
}

/// ARCH007: domain types must not import storage.
fn evaluate_arch007(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-domain",
        &["sddk-storage"],
        None,
    )
}

/// ARCH009: storage must not import engine.
fn evaluate_arch009(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-storage",
        &["sddk-engine"],
        None,
    )
}

/// ARCH010: the CLI must not import storage directly.
fn evaluate_arch010(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-cli",
        &["sddk-storage"],
        None,
    )
}

/// ARCH011: the vault must not import storage.
fn evaluate_arch011(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-vault",
        &["sddk-storage"],
        None,
    )
}

/// ARCH012: the testkit must not import storage.
fn evaluate_arch012(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    evaluate_forbidden_edge(
        rule,
        baseline,
        evaluated_at,
        "sddk-testkit",
        &["sddk-storage"],
        None,
    )
}

// ── ARCH008 ──────────────────────────────────────────────────────────────────

/// SDD-agnostic kernel: workflow_ir and workflow_run must not reference Phase or
/// CyclePath as Rust enum identifiers.
///
/// Uses a 4-pattern RegexSet over the scoped source files:
///   - \bPhase::        — type-qualified match
///   - \bCyclePath::     — type-qualified match
///   - \b(Explore|Specify|Design|Tasks|Apply|Verify|Archive)\s*::  — variant-qualified
///   - match\s+phase\s*\{  — match-on-string anti-pattern
///
/// YAML/JSON literals are excluded by file-extension filtering (scope only covers .rs files).
fn evaluate_arch008(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    use sddk_domain::EvaluatorKind;

    // 4-pattern RegexSet for Phase/CyclePath coupling
    static ARCH008_PATTERNS: std::sync::LazyLock<RegexSet> = std::sync::LazyLock::new(|| {
        RegexSet::new([
            r"\bPhase::",                                                  // type-qualified
            r"\bCyclePath::",                                              // type-qualified
            r"\b(Explore|Specify|Design|Tasks|Apply|Verify|Archive)\s*::", // variant-qualified
            r"match\s+phase\s*\{",                                         // match-on-string
        ])
        .expect("static regex")
    });

    let mut violations = Vec::new();

    // Walk each scope glob from the rule
    for glob_pattern in &rule.scope {
        let matching: Vec<_> = glob_match_files(glob_pattern);
        for file_path in matching {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (line_no, line) in content.lines().enumerate() {
                    let line_num = (line_no + 1) as u32;
                    if ARCH008_PATTERNS.is_match(line) {
                        violations.push(json!({
                            "file": file_path.to_string_lossy(),
                            "line": line_num,
                            "text": line,
                        }));
                    }
                }
            }
        }
    }

    let status = if violations.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "violations": violations,
            "count": violations.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH008 heuristic evaluator: scans scoped .rs files for Phase::/CyclePath:: patterns"
                .to_owned(),
        ),
    }
}

/// Simple glob matcher: expands "**/prefix" patterns recursively.
/// Returns absolute paths matching the pattern.
fn glob_match_files(pattern: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();

    // Only handle **/prefix patterns for now (simple form)
    if let Some(prefix) = pattern.strip_prefix("**/") {
        let prefix = prefix.trim_end_matches('/');
        // Walk the repo root looking for files ending with prefix
        if let Ok(cwd) = std::env::current_dir() {
            walk_matching_files(&cwd, prefix, &mut results);
        }
    }
    results
}

fn walk_matching_files(dir: &Path, suffix: &str, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target/, .git/, node_modules/
                let skip = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "target" || n == ".git" || n == "node_modules" || n == ".cargo")
                    .unwrap_or(false);
                if skip {
                    continue;
                }
                walk_matching_files(&path, suffix, results);
            } else if path.is_file() {
                let matches_suffix = path.to_str().map(|p| p.ends_with(suffix)).unwrap_or(false);
                if matches_suffix {
                    results.push(path);
                }
            }
        }
    }
}

// ── ARCH013 ──────────────────────────────────────────────────────────────────

/// dynamic_operators_require_capability_contract: stub for v1.29.0.
fn evaluate_arch013(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH013 substance (dynamic operator contracts) deferred to cycle 3".to_owned(),
        ),
    }
}

// ── ARCH014 ──────────────────────────────────────────────────────────────────

/// expansion_proposals_require_approval_receipt: stub for v1.29.0.
fn evaluate_arch014(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH014 substance (expansion proposal receipts) deferred to cycle 3".to_owned(),
        ),
    }
}

// ── ARCH015 ──────────────────────────────────────────────────────────────────

/// ir_events_must_not_emit_phase_strings: stub for v1.29.0.
fn evaluate_arch015(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH015 substance (IR event phase strings) deferred to cycle 3".to_owned(),
        ),
    }
}
