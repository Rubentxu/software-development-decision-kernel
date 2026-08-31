//! `comments_check` — scan production code for forbidden comment patterns.
//!
//! Implements the "Documentation Discipline" gate defined in
//! `prompts/sddk/phases/apply.md` §"Code Quality Standards" (L591-608) and
//! `prompts/sddk/phases/verify.md` §3.b.
//!
//! # Data-driven contract
//!
//! The patterns, languages, and excluded paths are NOT hard-coded in Rust.
//! They live in a YAML contract at
//! [`prompts/sddk/contracts/comments-rules.yaml`][contract], compiled into
//! the binary via `include_str!`, and overridable at runtime:
//!
//! - Per-invocation: `sddk dev check --rules /path/to/custom.yaml`
//! - Per-process: `SDDK_COMMENTS_RULES=/path/to/custom.yaml`
//!
//! Adding a new pattern = append an entry to the YAML. No recompile.
//!
//! [contract]: ../../../../../prompts/sddk/contracts/comments-rules.yaml
//!
//! # Multi-language scope
//!
//! The scanner understands comment syntax and string literals for Rust,
//! Python, JavaScript/TypeScript, Shell, TOML, YAML, Go, Java/Kotlin,
//! C-family, Ruby, and SQL. New languages = append to the YAML
//! `languages:` list.
//!
//! # Comment-only scanning
//!
//! For each line, the scanner extracts the *comment suffix* — the substring
//! after the first comment marker that is **not inside a string literal or
//! character literal**. Code lines that have no comment are skipped. This
//! avoids false positives on test fixtures like `"INC-001-3ef321c4"` (a
//! string literal) and CSS hex colors like `#0f1115` (also inside string
//! literals).
//!
//! The scanner is read-only and additive — it never modifies files.

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Path to the compiled-in default contract. Resolved relative to the
/// crate source at compile time and shipped in the bundle.
const DEFAULT_RULES_YAML: &str =
    include_str!("../../../../prompts/sddk/contracts/comments-rules.yaml");

// ─── Contract types ─────────────────────────────────────────────────────────

/// Top-level contract loaded from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct RulesContract {
    pub languages: Vec<LanguageSpec>,
    pub patterns: Vec<PatternSpec>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub skip_dirs: Vec<String>,
}

/// One supported language. Driven entirely by YAML config.
#[derive(Debug, Clone, Deserialize)]
pub struct LanguageSpec {
    pub id: String,
    pub extensions: Vec<String>,
    /// Single-line comment marker (`//`, `#`, `--`).
    pub line_comment: Option<String>,
    /// Multi-line comment opener.
    pub block_open: Option<String>,
    /// Language allows `r"..."` or `r#"..."#` style raw strings.
    #[serde(default)]
    pub supports_raw_strings: bool,
    /// Language allows `"""..."""` or `'''...'''` triple-quoted strings.
    #[serde(default)]
    pub supports_triple_quotes: bool,
    /// Language allows `` `...${expr}...` `` template literals.
    #[serde(default)]
    pub supports_template_literals: bool,
}

/// One forbidden pattern, as authored in the YAML contract.
#[derive(Debug, Clone, Deserialize)]
pub struct PatternSpec {
    pub name: String,
    pub needle: String,
    pub regex: String,
}

/// A pattern compiled into a `Regex`, ready for matching.
pub struct CompiledPattern {
    pub name: String,
    pub needle: String,
    pub regex: Regex,
}

impl CompiledPattern {
    fn from_spec(spec: &PatternSpec) -> Result<Self> {
        let regex = Regex::new(&spec.regex).with_context(|| {
            format!(
                "pattern `{}` has an invalid regex: {}",
                spec.name, spec.regex
            )
        })?;
        Ok(Self {
            name: spec.name.clone(),
            needle: spec.needle.clone(),
            regex,
        })
    }
}

// ─── Contract loading ───────────────────────────────────────────────────────

/// Load the active rules contract.
///
/// Resolution order (highest priority first):
/// 1. `SDDK_COMMENTS_RULES` env var (path to a YAML file).
/// 2. Compile-time default `prompts/sddk/contracts/comments-rules.yaml`
///    (embedded via `include_str!`).
pub fn load_rules() -> RulesContract {
    if let Ok(path) = std::env::var("SDDK_COMMENTS_RULES") {
        match load_rules_from_path(Path::new(&path)) {
            Ok(rules) => {
                eprintln!("comments_check: using rules from {path}");
                return rules;
            }
            Err(e) => {
                eprintln!(
                    "comments_check: failed to load SDDK_COMMENTS_RULES={path}: {e}. Falling back to compiled default."
                );
            }
        }
    }
    serde_saphyr::from_str(DEFAULT_RULES_YAML)
        .expect("compiled-in comments-rules.yaml is malformed")
}

/// Load rules from an explicit path (used by `--rules` CLI flag).
pub fn load_rules_from_path(path: &Path) -> Result<RulesContract> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading rules file {}", path.display()))?;
    serde_saphyr::from_str(&content)
        .with_context(|| format!("parsing rules file {}", path.display()))
}

/// Compile all patterns in a contract. Invalid regexes return errors.
pub fn compile_patterns(rules: &RulesContract) -> Result<Vec<CompiledPattern>> {
    rules
        .patterns
        .iter()
        .map(CompiledPattern::from_spec)
        .collect()
}

// ─── Language lookup ────────────────────────────────────────────────────────

/// Find the language spec that matches `path` based on its extension.
pub fn language_for<'a>(rules: &'a RulesContract, path: &Path) -> Option<&'a LanguageSpec> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())?
        .to_ascii_lowercase();
    rules
        .languages
        .iter()
        .find(|lang| lang.extensions.iter().any(|e| e == &ext))
}

// ─── Scanning ───────────────────────────────────────────────────────────────

/// A single violation found in the source.
#[derive(Debug, Clone)]
pub struct CommentViolation {
    pub file: PathBuf,
    pub line: u32,
    pub rule: String,
    pub snippet: String,
}

/// Run the scanner over a directory tree using the given rules.
pub fn scan(root: &Path, rules: &RulesContract) -> Result<Vec<CommentViolation>> {
    scan_with_filter(root, rules, &|_| true)
}

/// Run the scanner over files matching the given predicate.
pub fn scan_with_filter(
    root: &Path,
    rules: &RulesContract,
    predicate: &dyn Fn(&Path) -> bool,
) -> Result<Vec<CommentViolation>> {
    let patterns = compile_patterns(rules)?;
    let mut violations = Vec::new();
    walk(root, rules, &mut |path| {
        if !is_scannable(rules, path) {
            return;
        }
        if !predicate(path) {
            return;
        }
        if let Err(e) = scan_file(path, rules, &patterns, &mut violations) {
            eprintln!("warning: scan failed for {}: {}", path.display(), e);
        }
    })?;
    Ok(violations)
}

/// A set of line ranges in a single file.
#[derive(Debug, Default, Clone)]
pub struct AddedLines {
    pub ranges: Vec<(PathBuf, std::ops::RangeInclusive<u32>)>,
}

impl AddedLines {
    pub fn file_predicate(&self) -> Vec<PathBuf> {
        let mut seen = std::collections::BTreeSet::new();
        for (path, _) in &self.ranges {
            seen.insert(path.clone());
        }
        seen.into_iter().collect()
    }

    pub fn contains(&self, path: &Path, line_no: u32) -> bool {
        self.ranges
            .iter()
            .any(|(p, r)| p == path && r.contains(&line_no))
    }
}

/// Run the scanner restricted to lines that fall inside `added` ranges.
pub fn scan_added_lines(
    root: &Path,
    rules: &RulesContract,
    added: &AddedLines,
) -> Result<Vec<CommentViolation>> {
    let patterns = compile_patterns(rules)?;
    let mut violations = Vec::new();
    let predicate_files = added.file_predicate();
    for path_rel in predicate_files {
        let path = root.join(&path_rel);
        if !is_scannable(rules, &path) {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("warning: read failed for {}: {}", path.display(), e);
                continue;
            }
        };
        let _lang_name = language_for(rules, &path)
            .map(|l| l.id.clone())
            .unwrap_or_else(|| "unknown".into());
        for (idx, line) in content.lines().enumerate() {
            let line_no = (idx + 1) as u32;
            if !added.contains(&path_rel, line_no) {
                continue;
            }
            let Some((comment, _)) = extract_comment(line, language_for(rules, &path)) else {
                continue;
            };
            for pattern in patterns.iter() {
                if !pattern.needle.is_empty() && !comment.contains(&pattern.needle) {
                    continue;
                }
                if pattern.regex.is_match(comment) {
                    violations.push(CommentViolation {
                        file: path.clone(),
                        line: line_no,
                        rule: pattern.name.clone(),
                        snippet: line.trim().to_string(),
                    });
                    break;
                }
            }
        }
    }
    Ok(violations)
}

/// Compute the set of added lines between `git_ref` and HEAD for all
/// supported language extensions under `crates/`.
pub fn added_lines_since(
    root: &Path,
    rules: &RulesContract,
    git_ref: &str,
) -> Result<Option<AddedLines>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("diff")
        .arg("--unified=0")
        .arg("--diff-filter=ACMRT")
        .arg(format!("{git_ref}...HEAD"))
        .arg("--")
        .arg("crates/")
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "git diff failed for ref `{git_ref}`: {}",
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(Some(parse_unified_diff(&stdout, root, rules)))
}

/// Parse a unified diff with `--unified=0` into a set of added line ranges.
fn parse_unified_diff(diff: &str, root: &Path, rules: &RulesContract) -> AddedLines {
    let mut added = AddedLines::default();
    let mut current_path: Option<PathBuf> = None;
    let mut current_added: Vec<u32> = Vec::new();

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            // Flush previous hunk.
            if let Some(path) = current_path.take()
                && !current_added.is_empty()
            {
                let min = *current_added.iter().min().unwrap();
                let max = *current_added.iter().max().unwrap();
                added.ranges.push((path, min..=max));
            }
            let p = PathBuf::from(rest);
            // Keep only files that match a known language extension.
            if language_for(rules, &p).is_some() {
                current_path = Some(root.join(p));
            } else {
                current_path = None;
            }
            current_added.clear();
        } else if line.starts_with("@@") {
            if let Some(path) = current_path.as_ref()
                && !current_added.is_empty()
            {
                let min = *current_added.iter().min().unwrap();
                let max = *current_added.iter().max().unwrap();
                added.ranges.push((path.clone(), min..=max));
            }
            current_added.clear();
            // Parse "+new_start" from the hunk header.
            if let Some(plus) = line.split('+').nth(1) {
                let plus = plus.split(' ').next().unwrap_or("");
                if let Some((start_str, _)) = plus.split_once(',') {
                    if let Ok(start) = start_str.parse::<u32>() {
                        current_added.push(start);
                    }
                } else if let Ok(start) = plus.parse::<u32>() {
                    current_added.push(start);
                }
            }
        } else if line.starts_with("+")
            && current_path.is_some()
            && let Some(last) = current_added.last()
        {
            current_added.push(last + 1);
        }
    }
    if let Some(path) = current_path
        && !current_added.is_empty()
    {
        let min = *current_added.iter().min().unwrap();
        let max = *current_added.iter().max().unwrap();
        added.ranges.push((path, min..=max));
    }
    added
}

/// Resolve a git ref to the set of files changed in `root` since that ref
/// (file-level only — no line ranges).
#[allow(dead_code)]
pub fn files_changed_since(
    root: &Path,
    rules: &RulesContract,
    git_ref: &str,
) -> Result<Option<Vec<PathBuf>>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("diff")
        .arg("--name-only")
        .arg("--diff-filter=ACMRT")
        .arg(format!("{git_ref}...HEAD"))
        .arg("--")
        .arg("crates/")
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "git diff failed for ref `{git_ref}`: {}",
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths: Vec<PathBuf> = stdout
        .lines()
        .filter(|l| !l.is_empty() && language_for(rules, Path::new(l)).is_some())
        .map(|l| root.join(l))
        .collect();
    Ok(Some(paths))
}

fn is_scannable(rules: &RulesContract, path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if language_for(rules, path).is_none() {
        return false;
    }
    let path_str = path.to_string_lossy();
    !rules
        .exclude_paths
        .iter()
        .any(|dir| path_str.contains(dir.as_str()))
}

fn walk(root: &Path, rules: &RulesContract, visit: &mut dyn FnMut(&Path)) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // skip_dirs is an exact match against the leaf name. Allow both
            // "target" (relative form) and "/target/" (substring form) to
            // be configured — substring wins because `path_str.contains(...)`
            // is what excludes_paths uses, but skip_dirs short-circuits the
            // walker before descending.
            if rules
                .skip_dirs
                .iter()
                .any(|d| d == &name_str || d.trim_matches('/') == name_str.as_ref())
            {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                visit(&path);
            }
        }
    }
    Ok(())
}

/// Extract the comment suffix of a source line, language-aware.
///
/// Returns the substring after the first comment marker (line or block)
/// that is not inside a string or character literal. Returns `None` if the
/// line has no comment.
fn extract_comment<'a>(
    line: &'a str,
    lang: Option<&'a LanguageSpec>,
) -> Option<(&'a str, &'a str)> {
    let lang = lang?;
    let trimmed_start = line.trim_start();
    let marker = lang.line_comment.as_deref()?;
    // Pure-comment line.
    if trimmed_start.starts_with(marker) {
        return Some((line, marker));
    }
    if let Some(open) = lang.block_open.as_deref()
        && trimmed_start.starts_with(open)
    {
        return Some((line, open));
    }
    // Inline comment: find the marker not inside any string/char literal.
    let stripped = strip_string_literals(line, lang);
    if let Some(pos) = find_inline_comment_marker(&stripped, lang) {
        return Some((&line[pos..], marker));
    }
    None
}

/// Find the position of the first inline comment marker in `line`, ignoring
/// any occurrences inside string/char literals.
fn find_inline_comment_marker(line: &str, lang: &LanguageSpec) -> Option<usize> {
    let marker = lang.line_comment.as_deref()?;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        // Block comment opener (e.g. /*).
        if let Some(open) = lang.block_open.as_deref()
            && line[i..].starts_with(open)
        {
            return Some(i);
        }
        // Line comment marker (e.g. //, #, --).
        if line[i..].starts_with(marker) {
            // Make sure we are not inside an identifier — for "#" (Python etc.)
            // "#" can only be a comment if preceded by whitespace or start of line.
            if matches!(marker, "#" | "--") {
                let prev_char = if i == 0 {
                    None
                } else {
                    line[..i].chars().last()
                };
                if let Some(p) = prev_char
                    && (p.is_alphanumeric() || p == '_')
                {
                    continue;
                }
            }
            return Some(i);
        }
        // String/char literal — advance past it using char-based indices.
        if c == '"' || c == '\'' {
            let quote = c;
            let prev_char = if i == 0 {
                None
            } else {
                line[..i].chars().last()
            };
            let raw = lang.supports_raw_strings && matches!(prev_char, Some('r'));
            let mut triple = false;
            if lang.supports_triple_quotes
                && let Some(&(_, next_c)) = chars.peek()
                && next_c == quote
            {
                let mut peek = chars.clone();
                peek.next();
                if let Some(&(_, third_c)) = peek.peek()
                    && third_c == quote
                {
                    triple = true;
                }
            }
            if raw {
                if triple {
                    chars.next();
                    chars.next();
                    let closer: String = std::iter::repeat_n(quote, 3).collect();
                    while let Some((j, _)) = chars.next() {
                        if line[j..].starts_with(&closer) {
                            chars.next();
                            chars.next();
                            break;
                        }
                    }
                } else {
                    for (j, ch) in chars.by_ref() {
                        if ch == quote {
                            break;
                        }
                        let _ = j;
                    }
                }
            } else if triple {
                chars.next();
                chars.next();
                let closer: String = std::iter::repeat_n(quote, 3).collect();
                while let Some((j, ch)) = chars.next() {
                    if ch == '\\' {
                        chars.next();
                        continue;
                    }
                    if line[j..].starts_with(&closer) {
                        chars.next();
                        chars.next();
                        break;
                    }
                }
            } else {
                while let Some((j, ch)) = chars.next() {
                    if ch == '\\' {
                        chars.next();
                        continue;
                    }
                    if ch == quote {
                        break;
                    }
                    let _ = j;
                }
            }
            continue;
        }
        // Template literal (driven by supports_template_literals).
        if c == '`' && lang.supports_template_literals {
            while let Some((j, ch)) = chars.next() {
                if ch == '\\' {
                    chars.next();
                    continue;
                }
                if ch == '`' {
                    break;
                }
                let _ = j;
            }
            continue;
        }
    }
    None
}

/// Replace string/char literals with spaces, leaving code and comments intact.
fn strip_string_literals(line: &str, lang: &LanguageSpec) -> String {
    let bytes = line.as_bytes();
    let mut buf: Vec<u8> = line.bytes().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Raw string (r"..." / r#"..."#).
        if lang.supports_raw_strings
            && c == 'r'
            && i + 1 < bytes.len()
            && matches!(bytes[i + 1] as char, '"' | '#')
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while j < bytes.len() && (bytes[j] as char == '#') {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && (bytes[j] as char == '"') {
                let closer = format!("\"{}", "#".repeat(hashes));
                let closer_bytes = closer.as_bytes();
                j += 1;
                while j < bytes.len() {
                    if j + closer_bytes.len() <= bytes.len()
                        && &bytes[j..j + closer_bytes.len()] == closer_bytes
                    {
                        j += closer_bytes.len();
                        break;
                    }
                    buf[j] = b' ';
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        if c == '"' || c == '\'' {
            let quote = c;
            buf[i] = b' ';
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch == '\\' && i + 1 < bytes.len() {
                    buf[i] = b' ';
                    buf[i + 1] = b' ';
                    i += 2;
                    continue;
                }
                if ch == quote {
                    buf[i] = b' ';
                    i += 1;
                    break;
                }
                buf[i] = b' ';
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    String::from_utf8(buf).unwrap_or_else(|_| line.to_string())
}

fn scan_file(
    path: &Path,
    rules: &RulesContract,
    patterns: &[CompiledPattern],
    violations: &mut Vec<CommentViolation>,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let lang = language_for(rules, path);
    let _lang_name = lang
        .map(|l| l.id.clone())
        .unwrap_or_else(|| "unknown".into());
    for (idx, line) in content.lines().enumerate() {
        let Some((comment, _marker)) = extract_comment(line, lang) else {
            continue;
        };
        for pattern in patterns.iter() {
            if !pattern.needle.is_empty() && !comment.contains(&pattern.needle) {
                continue;
            }
            if pattern.regex.is_match(comment) {
                violations.push(CommentViolation {
                    file: path.to_path_buf(),
                    line: (idx + 1) as u32,
                    rule: pattern.name.clone(),
                    snippet: line.trim().to_string(),
                });
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared rules fixture: compiled-in default. Tests are independent
    /// of whether the YAML file exists on disk.
    fn rules() -> &'static RulesContract {
        use std::sync::OnceLock;
        static CACHED: OnceLock<RulesContract> = OnceLock::new();
        CACHED.get_or_init(|| serde_saphyr::from_str(DEFAULT_RULES_YAML).unwrap())
    }

    #[test]
    fn detects_deferred_to_cycle_with_space() {
        let v = scan_content(
            "// Full GuardExpr AST parsing deferred to cycle 3.",
            unique_path("rs", "test1"),
        );
        assert!(
            v.iter().any(|x| x.rule == "action_in_cycle"),
            "expected action_in_cycle hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_cycle_pointer_with_hyphen() {
        let v = scan_content(
            "// Kernel-pure workflow operators for cycle-16 runtime.",
            unique_path("rs", "test2"),
        );
        assert!(
            v.iter().any(|x| x.rule == "cycle_pointer_hyphen"),
            "expected cycle_pointer_hyphen hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_task_identifier() {
        let v = scan_content("/// See REQ-WF-RT-001\n", unique_path("rs", "test3"));
        assert!(
            v.iter().any(|x| x.rule == "task_identifier"),
            "expected task_identifier hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_waiver_id() {
        let v = scan_content(
            "/// ARCH003 waived (WV-0026-ARCH008-legacy-compat-seam).",
            unique_path("rs", "test4"),
        );
        assert!(
            v.iter().any(|x| x.rule == "task_identifier"),
            "expected task_identifier (waiver) hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_audit_marker() {
        let v = scan_content(
            "/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 4 unused variants.",
            unique_path("rs", "test5"),
        );
        assert!(
            v.iter().any(|x| x.rule == "audit_marker"),
            "expected audit_marker hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_action_in_cycle_stub() {
        let v = scan_content(
            "/// Map over a collection (stub in v1.29.0 — full semantics in cycle 3).",
            unique_path("rs", "test6"),
        );
        assert!(
            v.iter().any(|x| x.rule == "action_in_cycle"),
            "expected action_in_cycle hit, got {:?}",
            v
        );
    }

    #[test]
    fn detects_todo_placeholder() {
        let v = scan_content("// TODO: implement", unique_path("rs", "test7"));
        assert!(
            v.iter().any(|x| x.rule == "placeholder_marker"),
            "expected placeholder_marker hit, got {:?}",
            v
        );
    }

    #[test]
    fn does_not_flag_clean_comment() {
        let v = scan_content(
            "/// Compute the SHA-256 of the input bytes.",
            unique_path("rs", "test8"),
        );
        assert!(v.is_empty(), "unexpected violations: {:?}", v);
    }

    #[test]
    fn does_not_flag_string_literal_inc_id() {
        let v = scan_content(
            "assert!(id.starts_with(\"INC-001-3ef321c4\"));",
            unique_path("rs", "test9"),
        );
        assert!(
            v.is_empty(),
            "string literal should not be flagged, got {:?}",
            v
        );
    }

    #[test]
    fn does_not_flag_css_hex_color() {
        let v = scan_content(
            "    :root {{ --bg:#0f1115; --card:#181b21; }}",
            unique_path("rs", "test10"),
        );
        assert!(
            v.is_empty(),
            "CSS hex color should not be flagged, got {:?}",
            v
        );
    }

    #[test]
    fn flags_inline_comment_with_deferred_to_cycle() {
        let v = scan_content(
            "let x = 5; // stub for v1.29.0, deferred to cycle 3",
            unique_path("rs", "test11"),
        );
        assert!(
            v.iter().any(|x| x.rule == "action_in_cycle"),
            "inline comment should be flagged, got {:?}",
            v
        );
    }

    // ── Python ─────────────────────────────────────────────────────────────
    #[test]
    fn detects_python_comment_with_cycle_pointer() {
        let v = scan_content(
            "# Capability routing is deferred to cycle-17.",
            unique_path("py", "test_py1"),
        );
        assert!(
            v.iter()
                .any(|x| matches!(x.rule.as_str(), "cycle_pointer_hyphen" | "action_in_cycle")),
            "python # comment should be flagged, got {:?}",
            v
        );
    }

    #[test]
    fn does_not_flag_python_hash_in_string() {
        let v = scan_content("color = \"#0f1115\"", unique_path("py", "test_py2"));
        assert!(
            v.is_empty(),
            "python string literal with # should not be flagged, got {:?}",
            v
        );
    }

    // ── JavaScript ─────────────────────────────────────────────────────────
    #[test]
    fn detects_js_comment_with_task_id() {
        let v = scan_content(
            "// See REQ-WF-RT-001 for the runtime contract.",
            unique_path("js", "test_js1"),
        );
        assert!(
            v.iter().any(|x| x.rule == "task_identifier"),
            "JS // comment should be flagged, got {:?}",
            v
        );
    }

    // ── Shell ──────────────────────────────────────────────────────────────
    #[test]
    fn detects_shell_comment_with_todo() {
        let v = scan_content(
            "# TODO: handle the edge case",
            unique_path("sh", "test_sh1"),
        );
        assert!(
            v.iter().any(|x| x.rule == "placeholder_marker"),
            "shell # comment should be flagged, got {:?}",
            v
        );
    }

    // ── TOML ───────────────────────────────────────────────────────────────
    #[test]
    fn does_not_flag_toml_field_name_with_dash() {
        let v = scan_content(
            r#"requirement_ref = "REQ-001""#,
            unique_path("toml", "test_toml1"),
        );
        assert!(
            v.is_empty(),
            "TOML field assignment should not be flagged, got {:?}",
            v
        );
    }

    // ── Markdown (excluded by default) ─────────────────────────────────────
    #[test]
    fn skips_markdown_files() {
        let v = scan_content(
            "# Cycle 3 REQ-K3-002 acceptance scenario",
            unique_path("md", "test_md1"),
        );
        assert!(
            v.is_empty(),
            "markdown files should be skipped, got {:?}",
            v
        );
    }

    // ── YAML contract loadable from a custom path ─────────────────────────
    #[test]
    fn loads_default_rules_from_compiled_yaml() {
        let r = load_rules();
        assert!(
            r.languages.iter().any(|l| l.id == "rust"),
            "default rules must include rust"
        );
        assert!(
            r.patterns.iter().any(|p| p.name == "action_in_cycle"),
            "default rules must include action_in_cycle"
        );
    }

    #[test]
    fn loads_rules_from_custom_path() {
        let yaml = r#"
version: 1
schema: sddk/comments-rules/v1
languages:
  - id: rust
    extensions: [rs]
    line_comment: "//"
languages_patterns: []
exclude_paths: []
skip_dirs: []
"#;
        // (note: the test deliberately has a minimal contract; we expect
        //  a parse error because `languages_patterns` is not a field.
        //  This guards against accidentally accepting malformed YAML.)
        let tmp = std::env::temp_dir().join("sddk-comments-rules-bad.yaml");
        let _ = std::fs::write(&tmp, yaml);
        let result = load_rules_from_path(&tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(result.is_err(), "malformed YAML should fail to parse");
    }

    #[test]
    fn loads_minimal_valid_rules_from_path() {
        let yaml = r#"
version: 1
schema: sddk/comments-rules/v1
languages:
  - id: rust
    extensions: [rs]
    line_comment: "//"
    block_open: "/*"
    block_close: "*/"
    supports_raw_strings: true
    supports_triple_quotes: false
    supports_template_literals: false
patterns:
  - name: custom_marker
    description: "Project-specific marker."
    needle: "BADGE"
    regex: '\bBADGE-\d+\b'
exclude_paths: []
skip_dirs: []
"#;
        let tmp = std::env::temp_dir().join("sddk-comments-rules-good.yaml");
        let _ = std::fs::write(&tmp, yaml);
        let custom = load_rules_from_path(&tmp).expect("valid YAML should parse");
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(custom.patterns.len(), 1);
        assert_eq!(custom.patterns[0].name, "custom_marker");
        // Verify the gate actually fires for the custom rule, using the
        // custom contract (not the default rules) — this is the whole
        // point of the YAML contract.
        let v = scan_content_with(
            "// BADGE-123 awarded to this module",
            unique_path("rs", "test_custom"),
            &custom,
        );
        assert!(
            v.iter().any(|x| x.rule == "custom_marker"),
            "custom_marker should fire, got {:?}",
            v
        );
        // And the default rules do NOT have custom_marker — so scanning
        // with the default contract should produce no hits for the same
        // input, proving the override is real.
        let v = scan_content(
            "// BADGE-123 awarded to this module",
            unique_path("rs", "test_custom_default"),
        );
        assert!(
            v.is_empty(),
            "default contract should NOT fire custom_marker, got {:?}",
            v
        );
    }

    fn unique_path(ext: &str, prefix: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        PathBuf::from(format!("/tmp/{prefix}_{pid}_{n}.{ext}"))
    }

    fn scan_content(content: &str, path: PathBuf) -> Vec<CommentViolation> {
        scan_content_with(content, path, rules())
    }

    fn scan_content_with(
        content: &str,
        path: PathBuf,
        contract: &RulesContract,
    ) -> Vec<CommentViolation> {
        let mut v = Vec::new();
        let _ = std::fs::write(&path, content);
        let patterns = compile_patterns(contract).expect("compile contract patterns");
        let _ = scan_file(&path, contract, &patterns, &mut v);
        let _ = std::fs::remove_file(&path);
        v
    }
}
