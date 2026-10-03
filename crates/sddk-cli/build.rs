//! Build identity: which commit this binary was built from, and from where that
//! answer came.
//!
//! **Why this exists.** `sddk 2.5.3` and `sddk 2.5.3` are not necessarily the
//! same binary. The workspace does not bump between releases, so everything
//! committed after the last publication ships under the same version number as
//! the installed binary. Two artefacts can therefore declare the same version
//! and contain different code, and nothing in the product said so.
//!
//! **Why the identity comes from the caller, not from here.** This script was
//! measured, in a throwaway crate, across four scenarios, and three of them
//! were failures:
//!
//! - no `.git`: the build degrades correctly (exit 0, `unknown`/`absent`).
//! - `.git` without `rerun-if-changed`: cargo caches the script and the
//!   identity freezes at the *first* build.
//! - `rerun-if-changed` on `.git/HEAD` alone: still frozen, because `HEAD` does
//!   not change content when you commit — it stays `ref: refs/heads/<branch>`.
//! - the resolved ref also declared, with `packed-refs` present: frozen **and
//!   wrong**. It emitted a stale commit with every appearance of being right.
//!
//! That last one is why `SDDK_GIT_SHA`, set by whoever runs the build, is the
//! source of truth, and why `git` here is a *diagnostic* fallback that never
//! decides anything on its own. A detector that emits a stale value without
//! saying so is worse than no detector: its output is indistinguishable from
//! the correct one.
//!
//! Emitted:
//! - `SDDK_BUILD_SHA` — the commit, or `unknown`.
//! - `SDDK_BUILD_SHA_SOURCE` — `env` | `git` | `absent`.
//! - `SDDK_BUILD_DIRTY` — `true` | `false` | `unknown`; a commit alone does not
//!   identify content that had uncommitted changes on top of it.

use std::process::Command;

const UNKNOWN: &str = "unknown";

fn main() {
    println!("cargo:rerun-if-env-changed=SDDK_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SDDK_BUILD_DIRTY");

    let (sha, source) = match env_sha() {
        Some(sha) => (sha, "env"),
        None => match git_sha() {
            Some(sha) => (sha, "git"),
            None => (UNKNOWN.to_owned(), "absent"),
        },
    };

    // A caller that pins the commit is taken at its word about dirtiness too:
    // re-asking git here would describe *this* tree, not the tree that was
    // built, and the two are not the same thing.
    let dirty: &'static str = match source {
        "env" => match std::env::var("SDDK_BUILD_DIRTY").as_deref() {
            Ok("true") => "true",
            Ok("false") => "false",
            _ => UNKNOWN,
        },
        _ => match git_dirty() {
            Some(true) => "true",
            Some(false) => "false",
            None => UNKNOWN,
        },
    };

    println!("cargo:rustc-env=SDDK_BUILD_SHA={sha}");
    println!("cargo:rustc-env=SDDK_BUILD_SHA_SOURCE={source}");
    println!("cargo:rustc-env=SDDK_BUILD_DIRTY={dirty}");
}

fn env_sha() -> Option<String> {
    let raw = std::env::var("SDDK_GIT_SHA").ok()?;
    let sha = raw.trim().to_owned();
    if sha.is_empty() || !is_hex_sha(&sha) {
        return None;
    }
    Some(sha)
}

/// A full commit id is 40 hex characters. Abbreviated ids are accepted on the
/// way in but normalized to what git itself accepts, so that a typo in the
/// environment cannot become a plausible-looking identity.
fn is_hex_sha(s: &str) -> bool {
    s.len() >= 7 && s.len() <= 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn git_sha() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    is_hex_sha(&sha).then_some(sha)
}

fn git_dirty() -> Option<bool> {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(!out.stdout.is_empty())
}
