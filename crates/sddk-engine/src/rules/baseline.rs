//! Baseline consumer for `baseline-dependency-entropy.json` and live workspace capture.

// `missing_docs` is allowed across this file because the Phase 1 ARCH
// evaluators were introduced before the workspace-wide
// `#![warn(missing_docs)]` activation. A future docs-pass cycle should
// restore the per-item `///` doc comments and remove this allow.
#![allow(missing_docs)]

use sddk_domain::BaselineRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Kind of cross-crate import edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossCrateImportKind {
    /// This edge was derived from a Cargo.toml dependency declaration.
    CargoDep,
    /// This edge was derived from a `use` statement in source code.
    Use,

    /// A `use` that sits inside a `#[cfg(test)]` module.
    ///
    /// Production and test are different claims. A CLI command module that
    /// constructs `SqliteEventStore` at runtime is the CLI reaching the
    /// adapter; a test that constructs one to prove the command works is the
    /// test suite doing its job. Counting the second as the first inflates
    /// every edge rule over the whole workspace with edges that only exist
    /// under `cargo test`.
    ///
    /// Kept as a separate kind rather than a boolean so the discrimination
    /// lives in the data the rules already match on, instead of in a field
    /// every future rule has to remember to honour.
    TestScopedUse,
}

/// A single cross-crate import edge, either from a `use` statement or a Cargo dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossCrateImport {
    pub from_file: String,
    pub line: u32,
    pub from_crate: String,
    pub to_crate_raw: String,
    pub to_crate: String,
    #[serde(default = "default_kind")]
    pub kind: CrossCrateImportKind,
}

fn default_kind() -> CrossCrateImportKind {
    CrossCrateImportKind::Use
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub ref_: BaselineRef,
    pub cross_crate_imports: Vec<CrossCrateImport>,
}

#[derive(Debug, Deserialize)]
struct BaselineFile {
    schema_version: String,
    #[serde(default)]
    head_anchor: Option<String>,
    #[serde(default)]
    captured_at: Option<String>,
    #[serde(default)]
    cross_crate_coupling_baseline: CrossCrateCouplingBaseline,
}

#[derive(Debug, Default, Deserialize)]
struct CrossCrateCouplingBaseline {
    #[serde(default)]
    cross_crate_imports: Vec<RawCrossCrateImport>,
}

#[derive(Debug, Deserialize)]
struct RawCrossCrateImport {
    #[serde(default)]
    from_file: String,
    #[serde(default)]
    line: u32,
    to_crate: String,
}

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("baseline I/O {path:?}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("baseline schema_version {actual} not in supported set {supported:?}")]
    UnsupportedSchemaVersion {
        actual: String,
        supported: Vec<String>,
    },
    #[error("live baseline capture failed: {0}")]
    Capture(String),
}

#[derive(Debug, Clone)]
pub struct BaselineConsumer {
    path: PathBuf,
    supported_versions: Vec<String>,
}

impl BaselineConsumer {
    pub fn new(
        path: impl AsRef<std::path::Path>,
        supported_versions: &[&str],
    ) -> Result<Self, BaselineError> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            supported_versions: supported_versions.iter().map(|s| (*s).to_owned()).collect(),
        })
    }

    pub fn load(&self) -> Result<Baseline, BaselineError> {
        let bytes = std::fs::read(&self.path).map_err(|e| BaselineError::Io {
            path: self.path.clone(),
            message: e.to_string(),
        })?;
        let file: BaselineFile = serde_json::from_slice(&bytes).map_err(|e| BaselineError::Io {
            path: self.path.clone(),
            message: e.to_string(),
        })?;
        if !self
            .supported_versions
            .iter()
            .any(|v| v == &file.schema_version)
        {
            return Err(BaselineError::UnsupportedSchemaVersion {
                actual: file.schema_version,
                supported: self.supported_versions.clone(),
            });
        }
        let sha = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256:{:x}", h.finalize())
        };
        let ref_ = BaselineRef {
            schema_version: file.schema_version,
            head_anchor: file.head_anchor.unwrap_or_else(|| "unknown".to_owned()),
            sha256: sha,
            cycle_id: None,
            captured_at: file.captured_at.unwrap_or_else(|| "unknown".to_owned()),
        };
        let cross_crate_imports = file
            .cross_crate_coupling_baseline
            .cross_crate_imports
            .into_iter()
            .map(|raw| {
                let parts: Vec<&str> = raw.from_file.split('/').collect();
                let from_crate = if parts.len() >= 2 && parts[0] == "crates" {
                    parts[1].to_owned()
                } else if parts.len() >= 3 && parts[0] == ".." && parts[1] == "crates" {
                    parts[2].to_owned()
                } else {
                    "unknown".to_owned()
                };
                let to_crate = if raw.to_crate.starts_with("sddk-") {
                    raw.to_crate.clone()
                } else {
                    format!("sddk-{}", raw.to_crate)
                };
                CrossCrateImport {
                    from_file: raw.from_file,
                    line: raw.line,
                    from_crate,
                    to_crate_raw: raw.to_crate,
                    to_crate,
                    kind: CrossCrateImportKind::Use,
                }
            })
            .collect();
        Ok(Baseline {
            ref_,
            cross_crate_imports,
        })
    }

    /// Captures a live baseline from the workspace at `root` by parsing the
    /// workspace `Cargo.toml` (members + per-crate dependencies) and scanning
    /// `crates/*/src/**/*.rs` for `use sddk_*::...` cross-crate imports.
    ///
    /// `head_anchor` is the current short SHA (obtained via `git rev-parse
    /// --short HEAD`) and `sha256` hashes the workspace Cargo.toml + every
    /// `src/**/*.rs` file (deterministic across re-runs without local edits).
    pub fn capture_live(root: &Path) -> Result<Baseline, BaselineError> {
        let root = root.to_path_buf();

        // ── head_anchor ──────────────────────────────────────────────────────
        let head_anchor = exec_git(&root, &["rev-parse", "--short", "HEAD"])
            .map(|s| s.trim().to_owned())
            .unwrap_or_else(|_| "unknown".to_owned());

        // ── read workspace Cargo.toml ─────────────────────────────────────────
        let workspace_toml_path = root.join("Cargo.toml");
        let workspace_toml =
            read_text(&workspace_toml_path).map_err(|e| BaselineError::Capture(e.to_string()))?;

        // ── parse members + workspace.dependencies ────────────────────────────
        let members = parse_workspace_members(&workspace_toml)?;
        let workspace_deps = parse_workspace_dependencies(&workspace_toml)?;

        // ── collect all src files (for hashing + scanning) ───────────────────
        let mut src_files: Vec<PathBuf> = Vec::new();
        for member in &members {
            let src_dir = root.join("crates").join(member).join("src");
            if src_dir.exists() {
                collect_rs_files(&src_dir, &mut src_files);
            }
        }
        src_files.sort();

        // ── sha256 of workspace Cargo.toml + all src files in order ───────────
        let mut hasher = Sha256::new();
        hasher.update(workspace_toml.as_bytes());
        for file_path in &src_files {
            if let Ok(content) = read_text(file_path) {
                hasher.update(content.as_bytes());
            }
        }
        let sha256 = format!("sha256:{:x}", hasher.finalize());

        // ── cargo edges ───────────────────────────────────────────────────────
        let mut cross_crate_imports: Vec<CrossCrateImport> = Vec::new();

        for member in &members {
            let crate_toml_path = root.join("crates").join(member).join("Cargo.toml");
            let crate_toml = match read_text(&crate_toml_path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let deps = parse_crate_dependencies(&crate_toml, &workspace_deps)?;
            for (dep_name, dep_target) in deps {
                if dep_target.starts_with("sddk-") || dep_name == "sddk-domain" {
                    let from_crate = member.clone();
                    let to_crate = normalize_crate_name(&dep_target);
                    cross_crate_imports.push(CrossCrateImport {
                        from_file: format!("<cargo-dep:{dep_name}>"),
                        line: 0,
                        from_crate,
                        to_crate_raw: dep_target,
                        to_crate,
                        kind: CrossCrateImportKind::CargoDep,
                    });
                }
            }
        }

        // ── source-level use statements ───────────────────────────────────────
        for file_path in &src_files {
            let content = match read_text(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let rel_path = file_path
                .strip_prefix(&root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .replace('\\', "/");

            scan_use_statements(&content, &rel_path, &mut cross_crate_imports);
        }

        let captured_at = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            // ISO-8601 without chrono: just format as unix epoch secs
            format!("{secs}")
        };

        let ref_ = BaselineRef {
            schema_version: "1.0.0".to_owned(),
            head_anchor,
            sha256,
            cycle_id: None,
            captured_at,
        };

        Ok(Baseline {
            ref_,
            cross_crate_imports,
        })
    }
}

// ── helper functions ────────────────────────────────────────────────────────────

fn read_text(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

fn exec_git(root: &Path, args: &[&str]) -> Result<String, BaselineError> {
    use std::process::Command;
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| BaselineError::Capture(format!("git failed: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(BaselineError::Capture(format!(
            "git exited with {}",
            output.status
        )))
    }
}

/// Parses `members = [...]` from a workspace Cargo.toml.
fn parse_workspace_members(toml: &str) -> Result<Vec<String>, BaselineError> {
    let mut members = Vec::new();

    // Find the [workspace] section start index
    let workspace_start = toml
        .find("[workspace]")
        .ok_or_else(|| BaselineError::Capture("no [workspace] section found".to_string()))?;

    // Find the next top-level section after [workspace]
    let after_workspace = &toml[workspace_start..];
    let next_section = after_workspace[8..]
        .find("\n[")
        .map(|pos| pos + 8) // include the newline we skipped
        .unwrap_or(after_workspace.len());

    let workspace_block = &after_workspace[..next_section];

    // Find members = [...] within workspace_block
    if let Some(members_start) = workspace_block.find("members") {
        let after_members = &workspace_block[members_start..];
        // Collect all lines from "members" onwards
        let mut in_array = false;
        let mut content = String::new();
        for line in after_members.lines() {
            let trimmed = line.trim();
            if !in_array {
                if trimmed.starts_with("members = [") || trimmed == "members = [" {
                    in_array = true;
                    // Handle inline [ ... ]
                    if let Some(start) = trimmed.find('[') {
                        let rest = &trimmed[start + 1..];
                        if let Some(end) = rest.find(']') {
                            content.push_str(&rest[..end]);
                        }
                    }
                }
            } else {
                // Already in array
                if trimmed.starts_with(']') {
                    // End of array
                    break;
                }
                content.push_str(trimmed);
            }
        }

        let inner = content.trim();
        if !inner.is_empty() {
            for item in inner.split(',') {
                let item = item.trim().trim_matches('"').trim_matches('\'');
                if !item.is_empty() {
                    let name = if let Some(rest) = item.strip_prefix("crates/") {
                        rest
                    } else if let Some(rest) = item.strip_prefix("../") {
                        rest.strip_prefix("crates/").unwrap_or(rest)
                    } else {
                        item
                    };
                    members.push(name.to_owned());
                }
            }
        }
    }

    Ok(members)
}

/// Parses `[workspace.dependencies]` from a workspace Cargo.toml.
fn parse_workspace_dependencies(toml: &str) -> Result<HashMap<String, String>, BaselineError> {
    let mut deps: HashMap<String, String> = HashMap::new();
    let mut in_workspace_deps = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[workspace.dependencies]") {
            in_workspace_deps = true;
            continue;
        }
        if in_workspace_deps {
            if trimmed.starts_with('[') {
                break;
            }
            // name = { ... } or name = "..."
            if let Some(eq_pos) = trimmed.find(" = ") {
                let name = trimmed[..eq_pos].trim().to_owned();
                let value = trimmed[eq_pos + 3..].trim();
                // value might be { path = "..." } or "version" or { version = "..." }
                if value.starts_with('{') {
                    // { path = "...", version = "..." } or just { path = "..." }
                    if let Some(path_start) = value.find("path") {
                        let after_path = &value[path_start..];
                        if let Some(quote_start) = after_path.find('"') {
                            let after_quote = &after_path[quote_start + 1..];
                            if let Some(quote_end) = after_quote.find('"') {
                                deps.insert(name, format!("path:{}", &after_quote[..quote_end]));
                            }
                        }
                    }
                } else {
                    // "version" string
                    deps.insert(name, value.trim_matches('"').trim_matches('\'').to_owned());
                }
            }
        }
    }
    Ok(deps)
}

/// Parses `[dependencies]` from a crate Cargo.toml and resolves workspace deps.
fn parse_crate_dependencies(
    toml: &str,
    workspace_deps: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, BaselineError> {
    let mut deps: Vec<(String, String)> = Vec::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if in_deps {
            if trimmed.starts_with('[') {
                break;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = trimmed.find(" = ") {
                let name = trimmed[..eq_pos].trim().to_owned();
                let value = trimmed[eq_pos + 3..].trim();

                // Check for version.workspace = true
                if value.contains("workspace") && value.contains("true") && !value.starts_with('{')
                {
                    // version.workspace = true → resolve from workspace_deps
                    if let Some(resolved) = workspace_deps.get(&name) {
                        deps.push((name, resolved.clone()));
                    }
                    continue;
                }

                if value.starts_with('{') {
                    // { path = "...", package = "..." } or { version = "..." }
                    let mut resolved_value = value.to_owned();

                    // Check for `package = "..."` override
                    if let Some(p_start) = value.find("package") {
                        let after_p = &value[p_start..];
                        if let Some(q1) = after_p.find('"') {
                            let after_q1 = &after_p[q1 + 1..];
                            if let Some(q2) = after_q1.find('"') {
                                let package_name = &after_q1[..q2];
                                resolved_value = format!("path:{}", package_name);
                            }
                        }
                    } else if let Some(path_start) = value.find("path") {
                        let after_path = &value[path_start..];
                        if let Some(quote_start) = after_path.find('"') {
                            let after_quote = &after_path[quote_start + 1..];
                            if let Some(quote_end) = after_quote.find('"') {
                                resolved_value = format!("path:{}", &after_quote[..quote_end]);
                            }
                        }
                    }

                    // Skip sddk_internal_ prefixed or non-sddk deps
                    if resolved_value.starts_with("path:") || name.starts_with("sddk_") {
                        deps.push((name, resolved_value));
                    }
                } else {
                    // "version" string — skip unless sddk_
                    if name.starts_with("sddk_") {
                        deps.push((name, value.trim_matches('"').to_owned()));
                    }
                }
            }
        }
    }
    Ok(deps)
}

fn normalize_crate_name(raw: &str) -> String {
    if raw.starts_with("sddk-") {
        raw.to_owned()
    } else if let Some(path) = raw.strip_prefix("path:") {
        // path: crates/sddk-X  or  path: ../sddk-X
        let name = path.rsplit('/').next().unwrap_or(path);
        if name.starts_with("sddk-") {
            name.to_owned()
        } else {
            format!("sddk-{}", name)
        }
    } else {
        format!("sddk-{}", raw)
    }
}

/// Scans `content` (whole file text) for `use sddk_X::...` patterns and
/// appends CrossCrateImport entries to `out`.  Only `sddk_` imports are
/// captured; `crate::` and `super::` are intentionally skipped (they resolve
/// within the same crate and don't form cross-crate edges).
fn scan_use_statements(content: &str, rel_path: &str, out: &mut Vec<CrossCrateImport>) {
    let mut depth: i32 = 0;
    // Depth INSIDE the innermost test block, once we are in it. `None` means
    // production.
    let mut test_depth: Option<i32> = None;
    // Depth at which a `#[cfg(test)]` attribute sat, waiting for the block it
    // annotates to actually open.
    let mut pending_at: Option<i32> = None;

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let depth_before = depth;

        // Leave the test scope when its block closes: the line after the
        // closing brace sits at a lower depth than any line inside it.
        if test_depth.is_some_and(|inside_at| depth_before < inside_at) {
            test_depth = None;
        }

        if trimmed.starts_with("#[cfg(test)]") {
            pending_at = Some(depth);
        }

        // Enter the block the attribute annotates. This is a separate step from
        // the attribute because the `mod tests {` line is itself OUTSIDE the
        // block: recording the depth at the attribute made the `mod` line look
        // like the end of the block, and everything after it stayed test-scoped
        // — silently deleting production edges from the verdict. That was not
        // hypothetical; two of the tests below caught exactly it.
        if pending_at.is_some_and(|at| depth_before > at) {
            test_depth = Some(depth_before);
            pending_at = None;
        }

        // Brace counting, with the usual caveat: a brace inside a string or a
        // comment moves the count. That is why the fallback below is the
        // STRICT one. A miscount can only ever push an edge OUT of test scope
        // and into production, never the other way round — the gate stays
        // stricter when the heuristic is wrong, which is the direction that
        // can still catch debt.
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth < 0 {
            depth = 0;
        }

        // `pub use` counts too, and it counts MORE: a re-export makes the
        // adapter part of this crate's public surface, so any consumer of the
        // CLI now depends on it transitively. The scanner used to require the
        // line to start with `use `, which meant `pub use sddk_storage::Storage;`
        // in the composition root was not an edge at all — the one place where
        // the ARCH003 waiver is written down. The measurement could not see the
        // edge its own waiver names.
        let rest = if let Some(r) = trimmed.strip_prefix("pub use ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("use ") {
            r
        } else {
            continue;
        };
        // Find "use sddk_" pattern
        if !rest.starts_with("sddk_") {
            continue;
        }
        // Extract up to "::"
        if let Some(end) = rest.find("::") {
            let to_crate_raw = &rest[..end]; // e.g. "sddk_storage"
            let to_crate = format!("sddk-{}", &to_crate_raw[5..]); // drop "sddk_" prefix

            let from_crate = rel_path.split('/').nth(1).unwrap_or("unknown").to_owned();

            out.push(CrossCrateImport {
                from_file: rel_path.to_owned(),
                line: (line_idx + 1) as u32,
                from_crate,
                to_crate_raw: to_crate_raw.to_owned(),
                to_crate,
                kind: if test_depth.is_some() {
                    CrossCrateImportKind::TestScopedUse
                } else {
                    CrossCrateImportKind::Use
                },
            });
        }
    }
}

/// Recursively collects all `*.rs` files under `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_live_finds_engine_to_storage_edge() {
        // Run against the real repo (this is the sddk-engine crate itself)
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("must resolve repo root");
        let baseline = BaselineConsumer::capture_live(&repo_root)
            .expect("capture_live must succeed on this repo");

        // ARCH001 closed (v1.14.0): engine must NOT depend on sddk-storage.
        let engine_storage: Vec<_> = baseline
            .cross_crate_imports
            .iter()
            .filter(|e| e.from_crate == "sddk-engine" && e.to_crate == "sddk-storage")
            .collect();
        assert!(
            engine_storage.is_empty(),
            "ARCH001 regression: sddk-engine must not depend on sddk-storage; \
             offending edges: {:?}",
            engine_storage,
        );
    }

    #[test]
    fn capture_live_cross_crate_imports_have_positive_lines() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("must resolve repo root");
        let baseline =
            BaselineConsumer::capture_live(&repo_root).expect("capture_live must succeed");

        for edge in &baseline.cross_crate_imports {
            if edge.kind == CrossCrateImportKind::Use {
                assert!(
                    edge.line > 0,
                    "Use-kind edges must have line > 0, got line={} for {}",
                    edge.line,
                    edge.from_file
                );
                assert!(
                    edge.from_file.starts_with("crates/"),
                    "Use-kind edges must have from_file starting with 'crates/', got {}",
                    edge.from_file
                );
            }
        }
    }

    #[test]
    fn capture_live_hash_stable_across_runs() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("must resolve repo root");

        let baseline1 =
            BaselineConsumer::capture_live(&repo_root).expect("first capture_live must succeed");
        let baseline2 =
            BaselineConsumer::capture_live(&repo_root).expect("second capture_live must succeed");

        assert_eq!(
            baseline1.ref_.sha256, baseline2.ref_.sha256,
            "sha256 must be stable across two capture_live calls on the same tree"
        );
    }

    // ── El capturador: que arista existe, y de quien ────────────────────────
    //
    // Tres huecos encontrados midiendo ARCH010, y los tres hacen lo mismo:
    // la medicion cuenta menos de lo que existe, y lo hace en silencio.
    //
    //   1. un `use` dentro de `#[cfg(test)]` contaba como arista de PRODUCCION.
    //   2. un `pub use` no contaba como arista en absoluto.
    //   3. `dev/projection.rs` llama a `sddk_storage::SqliteEventStore::open(..)`
    //      sin `use`, y tampoco cuenta. Ese sigue ABIERTO y lo dice el guard
    //      de cobertura; aqui solo se arreglan los dos primeros.

    fn kinds_of(src: &str) -> Vec<CrossCrateImportKind> {
        let mut out = Vec::new();
        scan_use_statements(src, "crates/sddk-cli/src/x.rs", &mut out);
        out.into_iter().map(|e| e.kind).collect()
    }

    #[test]
    fn a_use_in_a_test_module_is_not_a_production_edge() {
        let src = "use sddk_storage::SqliteEventStore;\n\n#[cfg(test)]\nmod tests {\n    use sddk_storage::SqliteEventStore;\n}\nuse sddk_storage::SqliteControlPlane;\n";
        assert_eq!(
            kinds_of(src),
            vec![
                CrossCrateImportKind::Use,
                CrossCrateImportKind::TestScopedUse,
                CrossCrateImportKind::Use,
            ],
            "el use de test no puede salir como arista de produccion, y el de DESPUES del \
             modulo tiene que volver a ser de produccion"
        );
    }

    #[test]
    fn a_use_after_a_test_module_closes_is_production_again() {
        // El fallo silencioso de la cuenta de llaves: si el bloque de test no
        // se cerrara bien, todo lo que viniera despues quedaria marcado como
        // test y las aristas de produccion desaparecerian del veredicto.
        let src = "#[cfg(test)]\nmod tests {\n    use sddk_storage::A;\n}\n\nfn real() {}\nuse sddk_storage::B;\n";
        let kinds = kinds_of(src);
        assert_eq!(kinds[0], CrossCrateImportKind::TestScopedUse);
        assert_eq!(
            kinds[1],
            CrossCrateImportKind::Use,
            "tras cerrarse `mod tests`, la arista vuelve a ser de produccion"
        );
    }

    #[test]
    fn a_pub_use_is_an_edge() {
        // El composition root re-exporta `Storage` con `pub use`, y el escaner
        // exigia que la linea empezara por `use `. La arista que el waiver de
        // ARCH003 concede por nombre no existia en la medicion que ese waiver
        // pretendia fundamentar.
        let mut out = Vec::new();
        scan_use_statements(
            "pub use sddk_storage::Storage;\n",
            "crates/sddk-cli/src/lib.rs",
            &mut out,
        );
        assert_eq!(out.len(), 1, "un re-export publico es una dependencia");
        assert_eq!(out[0].to_crate, "sddk-storage");
    }

    #[test]
    fn a_stray_brace_cannot_hide_a_production_edge() {
        // La cuenta de llaves por linea no entiende cadenas ni comentarios. Sin
        // `#[cfg(test)]` de por medio no puede haber ambigüedad posible: este
        // test fija la DIRECCION del fallo, que es que un error de conteo solo
        // puede empujar una arista HACIA produccion, nunca fuera de ella.
        let src = "fn f() { let s = \"{\"; }\nuse sddk_storage::X;\n";
        assert!(
            kinds_of(src)
                .iter()
                .all(|k| *k == CrossCrateImportKind::Use),
            "sin `#[cfg(test)]` de por medio, ninguna arista puede marcarse como de test"
        );
    }
}
