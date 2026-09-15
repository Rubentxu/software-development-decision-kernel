// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_mutation/sandbox.rs — A3-S6 / AC6 disposable sandbox.
//
// The sandbox is an in-memory `path -> content` map. Mutations are applied to
// a *clone* of it and the clone is discarded. Nothing here touches the
// filesystem: no `std::fs`, no `std::io::Write`, no `File` (REQ-AC6-001).

use std::collections::BTreeMap;

use super::types::MutationInjection;

/// A disposable, in-memory copy of source files keyed by repo-relative path.
///
/// Ordering is deterministic (`BTreeMap`), so guard evaluation and receipts are
/// reproducible. The type deliberately exposes no persistence method.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutationSandbox {
    files: BTreeMap<String, String>,
}

impl MutationSandbox {
    /// An empty sandbox.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a sandbox from `(path, content)` pairs.
    pub fn from_sources<I, P, C>(sources: I) -> Self
    where
        I: IntoIterator<Item = (P, C)>,
        P: Into<String>,
        C: Into<String>,
    {
        let mut files = BTreeMap::new();
        for (p, c) in sources {
            files.insert(p.into(), c.into());
        }
        Self { files }
    }

    /// Number of files in the sandbox.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the sandbox is empty.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Whether a path is present.
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Borrow the content of a path.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    /// Iterate `(path, content)` in deterministic path order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Insert or replace a path's content.
    pub fn insert(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }
}

/// Apply a mutation injection to `sandbox` at `target_path`.
///
/// Returns whether the injection actually applied:
/// - `AppendLine` / `PrependLine` always apply (creating the target if absent).
/// - `InsertBeforeFirstLineContaining` applies only when `needle` is present.
pub fn apply_injection(
    sandbox: &mut MutationSandbox,
    target_path: &str,
    injection: &MutationInjection,
) -> bool {
    let existing = sandbox.get(target_path).unwrap_or("").to_string();
    match injection {
        MutationInjection::AppendLine(line) => {
            let mut next = existing;
            if !next.is_empty() && !next.ends_with('\n') {
                next.push('\n');
            }
            next.push_str(line);
            next.push('\n');
            sandbox.insert(target_path, next);
            true
        }
        MutationInjection::PrependLine(line) => {
            let next = format!("{line}\n{existing}");
            sandbox.insert(target_path, next);
            true
        }
        MutationInjection::InsertBeforeFirstLineContaining { needle, line } => {
            let mut out = String::with_capacity(existing.len() + line.len() + 1);
            let mut applied = false;
            for src_line in existing.split_inclusive('\n') {
                if !applied && src_line.contains(needle.as_str()) {
                    out.push_str(line);
                    out.push('\n');
                    applied = true;
                }
                out.push_str(src_line);
            }
            if applied {
                sandbox.insert(target_path, out);
            }
            applied
        }
    }
}

/// Evaluate a guard over a sandbox and collect hits (REQ-AC6-008..010).
pub fn evaluate_guard(
    sandbox: &MutationSandbox,
    scope: &super::types::GuardScope,
    check: &super::types::GuardCheck,
) -> Vec<super::types::GuardHit> {
    use super::types::GuardCheck;
    let mut hits = Vec::new();
    for (path, content) in sandbox.iter() {
        if !scope.matches(path) {
            continue;
        }
        match check {
            GuardCheck::ForbiddenLines { forbidden } => {
                for (idx, line) in content.lines().enumerate() {
                    if forbidden.iter().any(|f| line.contains(f.as_str())) {
                        hits.push(super::types::GuardHit {
                            path: path.to_string(),
                            line: idx as u32 + 1,
                            matched: line.trim().to_string(),
                        });
                    }
                }
            }
            GuardCheck::MaxOccurrences {
                pattern,
                max_allowed,
            } => {
                let mut seen = 0usize;
                for (idx, line) in content.lines().enumerate() {
                    if line.contains(pattern.as_str()) {
                        seen += 1;
                        if seen > *max_allowed {
                            hits.push(super::types::GuardHit {
                                path: path.to_string(),
                                line: idx as u32 + 1,
                                matched: line.trim().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    hits
}
