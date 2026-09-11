// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
//! SPIKE AX-S5 — agent asset static scanner.
//!
//! Question (SPIKES.md AX-S5): can we detect deprecated command
//! mentions, raw store/table references, authority language,
//! duplicated prompt fragments and unregistered examples in
//! Markdown/YAML/Rust literals — with a false-positive rate low
//! enough to keep advisory?
//!
//! Method: pure scanner over `agents/` and `prompts/` Markdown assets
//! (no FS side effects; paths are passed in as content). Five
//! detectors, each returning findings with file/line evidence. Then
//! measure on the real asset corpus and record the FP rate in the
//! findings doc.
//!
//! Findings: docs/architecture/spikes/AX-S5-agent-asset-static-scanner.md

/// One scanner finding: location + rule + advisory snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Detector that produced the finding.
    pub rule: &'static str,
    /// Asset-relative path (or joined paths for cross-file rules).
    pub file: String,
    /// 1-based line (0 for cross-file rules).
    pub line: usize,
    /// Trimmed offending line (or summary).
    pub snippet: String,
}

/// Deterministic byte offset -> (line, col)-free helper: line number of
/// a needle within file content.
/// 1-based line number of a byte offset.
fn line_of(content: &str, needle_offset: usize) -> usize {
    content[..needle_offset].matches('\n').count() + 1
}

/// Trimmed full line around an offset, truncated to ~120 chars.
fn snippet_around(content: &str, offset: usize) -> String {
    let start = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = content[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or(content.len());
    let mut s = content[start..end].trim().to_string();
    if s.len() > 120 {
        s.truncate(117);
        s.push_str("...");
    }
    s
}

// ---------------------------------------------------------------------
// Rule 1: deprecated / renamed names (AGENTS.md §2.0 normalization:
// "SDD-kernel" -> SDDK, and known legacy CLI spellings).
// ---------------------------------------------------------------------

/// Legacy namespace spellings banned by AGENTS.md §2.0.
pub const DEPRECATED_NAMES: &[&str] = &["SDD-kernel", "sddk-kernel", "SDD-Kernel"];

/// Rule 1: deprecated/renamed namespace spellings.
pub fn scan_deprecated_names(file: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for name in DEPRECATED_NAMES {
        let mut from = 0;
        while let Some(pos) = content[from..].find(name) {
            let abs = from + pos;
            out.push(Finding {
                rule: "deprecated_name",
                file: file.into(),
                line: line_of(content, abs),
                snippet: snippet_around(content, abs),
            });
            from = abs + name.len();
        }
    }
    out
}

// ---------------------------------------------------------------------
// Rule 2: raw store/table references inside agent-facing prose (the
// vault/runtime separation, SPEC-002: agents must not speak SQL or
// point at store internals).
// ---------------------------------------------------------------------

/// Raw persistence markers agents-facing prose must not contain.
pub const STORE_MARKERS: &[&str] = &[
    "INSERT INTO",
    "SELECT * FROM",
    "CREATE TABLE",
    "sqlite://",
    ".db`",
];

/// Rule 2: raw store/table references in Markdown assets.
pub fn scan_raw_store_references(file: &str, content: &str) -> Vec<Finding> {
    if !file.ends_with(".md") {
        return Vec::new();
    }
    let mut out = Vec::new();
    for marker in STORE_MARKERS {
        let mut from = 0;
        while let Some(pos) = content[from..].find(marker) {
            let abs = from + pos;
            out.push(Finding {
                rule: "raw_store_reference",
                file: file.into(),
                line: line_of(content, abs),
                snippet: snippet_around(content, abs),
            });
            from = abs + marker.len();
        }
    }
    out
}

// ---------------------------------------------------------------------
// Rule 3: authority language that contradicts the capability model
// (SPEC-013..018: agents declare skills; capabilities gate side
// effects; prose must not grant them).
// ---------------------------------------------------------------------

/// Phrases that grant authority prose cannot grant (SPEC-013..018).
pub const AUTHORITY_PHRASES: &[&str] = &[
    "you are authorized to bypass",
    "ignore the invariant",
    "skip verification",
    "override the kernel invariant",
];

/// Rule 3: capability-bypassing authority language.
pub fn scan_authority_language(file: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for phrase in AUTHORITY_PHRASES {
        let mut from = 0;
        let lower = content.to_lowercase();
        while let Some(pos) = lower[from..].find(phrase) {
            let abs = from + pos;
            out.push(Finding {
                rule: "authority_language",
                file: file.into(),
                line: line_of(content, abs),
                snippet: snippet_around(content, abs),
            });
            from = abs + phrase.len();
        }
    }
    out
}

// ---------------------------------------------------------------------
// Rule 4: duplicated prompt fragments (exact normalized line blocks of
// >= 4 lines appearing in more than one file).
// ---------------------------------------------------------------------

/// Rule 4: identical 4+ normalized-line blocks across files.
pub fn scan_duplicated_fragments(files: &[(&str, &str)]) -> Vec<Finding> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, Vec<String>> = HashMap::new();
    for (file, content) in files {
        let lines: Vec<&str> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#') && l.len() > 20)
            .collect();
        if lines.len() < 4 {
            continue;
        }
        for w in lines.windows(4) {
            let key = w.join("\n");
            seen.entry(key).or_default().push((*file).to_string());
        }
    }
    let mut out = Vec::new();
    let mut reported: std::collections::HashSet<String> = Default::default();
    for (key, holders) in seen {
        if holders.len() > 1 {
            let unique: std::collections::BTreeSet<_> = holders.iter().collect();
            if unique.len() > 1 && reported.insert(key) {
                out.push(Finding {
                    rule: "duplicated_fragment",
                    file: unique
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    line: 0,
                    snippet: "4+ identical normalized lines across files".into(),
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Rule 5: unregistered CLI examples — `sddk <cmd>` mentions in prose
// whose first token is not a registered top-level command name.
// ---------------------------------------------------------------------

/// Rule 5: `sddk <cmd>` prose examples not in the command surface.
pub fn scan_unregistered_examples(file: &str, content: &str, commands: &[&str]) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = content[from..].find("sddk ") {
        let abs = from + pos;
        let rest = &content[abs + 5..];
        let word: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if !word.is_empty() && !commands.contains(&word.as_str()) {
            out.push(Finding {
                rule: "unregistered_example",
                file: file.into(),
                line: line_of(content, abs),
                snippet: snippet_around(content, abs),
            });
        }
        from = abs + 5;
    }
    out
}

/// Canonical top-level commands (mirrors command_spec table, pinned by
/// the clap sync guard — see command_spec_tests).
/// Canonical top-level commands (mirrors command_spec table).
pub const TOP_LEVEL_COMMANDS: &[&str] = &[
    "adopt",
    "agent-help",
    "agent-result",
    "analytics",
    "artifact",
    "capability",
    "cycle",
    "dev",
    "docs",
    "generate",
    "help",
    "inventory",
    "knowledge",
    "ledger",
    "lint",
    "plan",
    "release",
    "run",
    "run-view",
    "target",
    "uat",
    "vault",
    "verify",
    "version",
];

/// Convenience: run all detectors over a corpus.
/// Convenience: run all detectors over a corpus.
pub fn scan_all(files: &[(&str, &str)]) -> Vec<Finding> {
    let mut out = Vec::new();
    for (file, content) in files {
        out.extend(scan_deprecated_names(file, content));
        out.extend(scan_raw_store_references(file, content));
        out.extend(scan_authority_language(file, content));
        out.extend(scan_unregistered_examples(
            file,
            content,
            TOP_LEVEL_COMMANDS,
        ));
    }
    out.extend(scan_duplicated_fragments(files));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_deprecated_names() {
        let f = scan_deprecated_names("a.md", "Uses SDD-kernel naming here.");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "deprecated_name");
        assert_eq!(f[0].line, 1);
    }

    #[test]
    fn store_references_only_in_markdown() {
        assert!(scan_raw_store_references("a.rs", "let q = \"SELECT * FROM cycles\";").is_empty());
        let f = scan_raw_store_references("a.md", "run `SELECT * FROM cycles` to inspect");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "raw_store_reference");
    }

    #[test]
    fn authority_language_case_insensitive() {
        let f = scan_authority_language("a.md", "Agents may Ignore the invariant under load");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "authority_language");
    }

    #[test]
    fn duplicated_fragments_across_files() {
        let block = "line one of the shared block\nline two of the shared block\nline three of the shared block\nline four of the shared block\n";
        let files = vec![("a.md", block), ("b.md", block)];
        let f = scan_duplicated_fragments(&files);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "duplicated_fragment");
    }

    #[test]
    fn unregistered_examples_catch_stale_commands() {
        let f = scan_unregistered_examples(
            "a.md",
            "Run `sddk frobnicate --all` first.",
            TOP_LEVEL_COMMANDS,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "unregistered_example");
        // registered command passes
        assert!(
            scan_unregistered_examples("a.md", "Run `sddk dev install` now.", TOP_LEVEL_COMMANDS)
                .is_empty()
        );
    }

    #[test]
    fn scan_all_runs_every_detector() {
        let files = vec![("x.md", "See SDD-kernel docs. Try `sddk legacy-cmd`.")];
        let rules: Vec<&str> = scan_all(&files).iter().map(|f| f.rule).collect();
        assert!(rules.contains(&"deprecated_name"));
        assert!(rules.contains(&"unregistered_example"));
    }
}
